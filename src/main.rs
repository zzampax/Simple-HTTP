use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use colored::Colorize;
use json::{self, JsonValue};
use rusqlite::Connection;
use sha256::digest;
use std::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use urlencoding;

fn dbconn() -> Connection {
    return Connection::open("users.db").unwrap();
}

async fn check_template(contents: &mut String, userdata: JsonValue) -> String {
    for (key, value) in userdata.entries() {
        let key: String = key.to_string();
        let value: String = value.to_string();
        let key: String = format!("&{{{}}}", key);
        *contents = contents.replace(&key, &value);
    }

    return contents.to_string();
}

async fn get_userdata(token: &str) -> JsonValue {
    // Select user data from database using token, if token is invalid return empty JSON
    let dbconn: Connection = dbconn();

    let dbuser: String = match dbconn.query_row(
        "SELECT email FROM tokens WHERE token = ?1",
        [token],
        |row| row.get(0),
    ) {
        Ok(user) => user,
        Err(_) => "".to_string(),
    };

    // if user exists, return user data, else return empty JSON
    let user: JsonValue = if dbuser.is_empty() {
        json::parse("{}").unwrap()
    } else {
        json::parse(&format!(
            r#"{{
                "email": "{}",
                "token": "{}"
            }}"#,
            dbuser, token
        ))
        .unwrap()
    };

    return user;
}

async fn auth_token(token: &str) -> bool {
    let user_exists: bool = get_userdata(token).await != json::parse("{}").unwrap();
    return user_exists;
}

async fn api_messages() -> String {
    let mut messages: String = "".to_string();
    let dbconn: Connection = dbconn();

    let dbmessages: Vec<rusqlite::Result<(String, String, String)>> = dbconn
        .prepare("SELECT email, message, datetime FROM messages")
        .unwrap()
        .query_map([], |row| {
            Ok((
                row.get(0).unwrap(),
                row.get(1).unwrap(),
                row.get(2).unwrap(),
            ))
        })
        .unwrap()
        .collect();

    // format and send as JSON
    for message in dbmessages {
        let (token, message, datetime) = message.unwrap();
        let user: JsonValue = get_userdata(&token).await;
        messages += &format!(
            r#"{{
                "email": "{}",
                "message": "{}",
                "datetime": "{}"
            }},"#,
            user["email"], message, datetime
        );
    }
    messages.pop(); // remove trailing comma

    return format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n[{}]",
        messages
    );
}

async fn get_404() -> String {
    let contents: String = fs::read_to_string("pages/404.html").unwrap();
    return format!("HTTP/1.1 404 NOT FOUND\r\n\r\n{}", contents);
}

async fn get_index() -> String {
    let contents: String = fs::read_to_string("pages/index.html").unwrap();
    return format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);
}

async fn get_login() -> String {
    let contents: String = fs::read_to_string("pages/login.html").unwrap();
    return format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);
}

fn get_logout() -> String {
    // delete cookie from browser
    return format!("HTTP/1.1 301 OK\r\nSet-Cookie: token=; Max-Age=0\r\nLocation: /\r\nContent-Length: 0\r\n\r\n");
}

async fn get(mut path: String, _headers: Vec<&str>) -> String {
    let mut _query_string: String = "".to_string();
    //check for query string
    if path.contains("?") {
        _query_string = path.split("?").collect::<Vec<&str>>()[1].to_string();
        path = path.split("?").collect::<Vec<&str>>()[0].to_string();
    }

    let mut token: &str = "";
    for header in _headers {
        if header.starts_with("Cookie: token=") {
            token = header.split("token=").collect::<Vec<&str>>()[1];
        }
    }
    println!("Sha256 token: {}", token.cyan());
    let auth: bool = auth_token(&token).await;

    let mut contents: String = match path.as_str() {
        "/" => {
            if auth {
                get_index().await
            } else {
                get_login().await
            }
        }
        "/login" => {
            if auth {
                get_index().await
            } else {
                get_login().await
            }
        }
        "/logout" => get_logout(),
        "/api/v1/messages" => {
            if auth {
                api_messages().await
            } else {
                return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
            }
        }
        _ => get_404().await,
    };

    contents = check_template(&mut contents, get_userdata(token).await).await;
    return contents;
}

async fn post_login(dbconn: Connection, params: Vec<&str>) -> String {
    let mut email: &str = "";
    let mut password: &str = "";
    for param in params {
        let key_value: Vec<&str> = param.split('=').collect();
        if key_value.len() == 2 {
            let key: &str = key_value[0].trim();
            let value: &str = key_value[1].trim();
            match key {
                "email" => email = value,
                "password" => password = value,
                _ => (),
            }
        }
    }

    if email.is_empty() || password.is_empty() {
        return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request";
    }

    let to_encode: String = URL_SAFE.encode(
        json::object! {
            email: email,
            password: digest(password)
        }
        .dump()
        .as_bytes(),
    );

    // check if user exists, if not, create user
    let user_exists: bool = dbconn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM users WHERE email = ?1)",
            &[email],
            |row| row.get(0),
        )
        .unwrap();
    if !user_exists {
        dbconn
            .execute(
                "INSERT INTO users (email, password) VALUES (?1, ?2)",
                &[&email, &digest(password).as_str()],
            )
            .unwrap();
    } else {
        let user: String = dbconn
            .query_row(
                "SELECT password FROM users WHERE email = ?1",
                &[email],
                |row| row.get(0),
            )
            .unwrap();
        if user != digest(password) {
            return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
        }
    }

    // create Token (insert it and return it)
    let token: String = dbconn
        .query_row(
            "SELECT token FROM tokens WHERE email = ?1 AND timestamp > datetime('now', '-1 day')",
            &[email],
            |row| row.get(0),
        )
        .unwrap_or_else(|_| {
            let token: String = digest(&to_encode);
            dbconn
                .execute(
                    "INSERT INTO tokens (email, token) VALUES (?1, ?2)",
                    &[&email, &token.as_str()],
                )
                .unwrap();
            token
        });

    return format!(
        "HTTP/1.1 301 OK\r\nSet-Cookie: token={}\r\nLocation: /\r\nContent-Length: 0\r\n\r\n",
        &token
    );
}

async fn post_message(params: Vec<&str>, sha256_token: &str) -> String {
    if sha256_token.is_empty() {
        return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
    }

    let decoded: JsonValue = get_userdata(sha256_token).await;

    if decoded["email"].is_null() {
        return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
    }

    // find message in params and decode it using form_urlencoded
    let mut message: String = "".to_string();
    for param in params {
        let key_value: Vec<&str> = param.split('=').collect();
        if key_value.len() == 2 {
            let key: &str = key_value[0].trim();
            let value: &str = key_value[1].trim();
            if key == "message" {
                message = value.replace("+", " ");
            }
        }
    }

    if message.is_empty() {
        return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request";
    }

    dbconn()
        .execute(
            "INSERT INTO messages (email, message) VALUES (?1, ?2)",
            &[&decoded["email"].as_str().unwrap(), &message.as_str()],
        )
        .unwrap();

    println!(
        "{}",
        format!("{} says: {}", decoded["email"], message).bright_yellow()
    );

    return format!("HTTP/1.1 301 OK\r\nLocation: /\r\nContent-Length: 0\r\n\r\n");
}

async fn post(path: String, headers: Vec<&str>, body: &str) -> String {
    let body: String = urlencoding::decode(body).unwrap().to_string();
    let params: Vec<&str> = body
        .split("&")
        .map(|param: &str| param.trim_end_matches('\0'))
        .collect();
    let mut sha256_token: &str = "";
    for header in headers {
        if header.starts_with("Cookie: token=") {
            sha256_token = header.split("token=").collect::<Vec<&str>>()[1];
        }
    }
    println!("Sha256 token: {}", sha256_token.cyan());

    match path.as_str() {
        "/login" => post_login(dbconn(), params).await,
        "/message" => post_message(params, sha256_token).await,
        _ => return "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string() + "404 Not Found",
    }
}

async fn handle_connection(mut socket: tokio::net::TcpStream) {
    let mut buffer: [u8; 16384] = [0; 16384];
    socket.read(&mut buffer).await.unwrap();
    println!(
        "\nNew connection from {}",
        socket.peer_addr().unwrap().to_string().red()
    );

    let string_buffer: std::borrow::Cow<str> = String::from_utf8_lossy(&buffer);
    let mut lines: std::str::Lines = string_buffer.lines();

    let request_line: &str = lines.next().unwrap();
    // split the request line into three variables
    let mut request_line: std::str::SplitWhitespace = request_line.split_whitespace();
    println!(
        "Request: {}",
        request_line.clone().collect::<Vec<&str>>().join(" ").cyan()
    );

    let (method, path, _version) = (
        request_line.next().unwrap().to_string(),
        request_line.next().unwrap().to_string(),
        request_line.next().unwrap().to_string(),
    );
    let headers: Vec<&str> = lines
        .clone()
        .take_while(|line: &&str| !line.is_empty())
        .collect();

    let body: &str = lines.last().unwrap();

    println!("Method: {}, Path: {}", method.green(), path.yellow());

    if let Some(accept_header) = headers
        .iter()
        .find(|&header| header.starts_with("Accept: "))
    {
        if accept_header.contains("image") && path.contains("favicon.ico") {
            println!("Sending: {}", "favicon.ico".yellow());
            let response: String =
                "HTTP/1.1 200 OK\r\nContent-Type: image/x-icon\r\n\r\n".to_string();
            socket.write_all(response.as_bytes()).await.unwrap();
            let favicon: Vec<u8> = fs::read("pages/favicon.ico").unwrap();
            socket.write_all(&favicon).await.unwrap();
            return;
        }
    }

    let response: String = match method.as_str() {
        "GET" => get(path, headers).await,
        "POST" => post(path, headers, body).await,
        _ => "HTTP/1.1 405 METHOD NOT ALLOWED\r\n\r\n".to_string() + "405 Method Not Allowed",
    };

    socket.try_write(response.as_bytes()).unwrap();
}

#[tokio::main]
async fn main() {
    dbconn()
        .execute(
            "CREATE TABLE IF NOT EXISTS users (
                    email TEXT PRIMARY KEY,
                    password TEXT NOT NULL
            );",
            [],
        )
        .unwrap();

    dbconn()
        .execute(
            "CREATE TABLE IF NOT EXISTS messages (
                    message_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    email TEXT NOT NULL,
                    message TEXT,
                    datetime DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(email) REFERENCES users(email)
                );",
            [],
        )
        .unwrap();

    dbconn()
        .execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                    email TEXT PRIMARY KEY,
                    token TEXT NOT NULL,
                    timestamp DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(email) REFERENCES users(email)
                );",
            [],
        )
        .unwrap();

    let mut port: i32 = 3000;
    loop {
        match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(listener) => {
                println!("\n --> Server running on port {}! <--", port);

                loop {
                    let (socket, _) = listener.accept().await.unwrap();
                    tokio::spawn(handle_connection(socket));
                }
            }
            Err(_) => {
                println!("Port {} is in use, trying next port...", port);
                if port == 3010 {
                    println!("All safe ports are in use, exiting...");
                    break;
                }
                port += 1;
            }
        }
    }
}
