use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use colored::Colorize;
use json::{self, JsonValue};
use rusqlite::Connection;
use sha256::digest;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use urlencoding;
use uuid::Uuid;

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
        let (email, message, datetime) = message.unwrap();
        messages += &format!(
            r#"{{
                "email": "{}",
                "message": "{}",
                "datetime": "{}"
            }},"#,
            email, message, datetime
        );
    }
    messages.pop(); // remove trailing comma

    return format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n[{}]",
        messages
    );
}

async fn get_404() -> String {
    let contents: String = fs::read_to_string("pages/404.html").await.unwrap();
    return format!("HTTP/1.1 404 NOT FOUND\r\n\r\n{}", contents);
}

async fn get_index() -> String {
    let contents: String = fs::read_to_string("pages/index.html").await.unwrap();
    return format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);
}

async fn get_login() -> String {
    let contents: String = fs::read_to_string("pages/login.html").await.unwrap();
    return format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);
}

fn post_logout() -> String {
    // delete cookie from browser
    return format!("HTTP/1.1 301 OK\r\nSet-Cookie: token=; Max-Age=0\r\nLocation: /\r\nContent-Length: 0\r\n\r\n");
}

async fn get(mut path: String, headers: Vec<(String, String)>) -> String {
    let mut _query_string: String = "".to_string();
    //check for query string
    if path.contains("?") {
        _query_string = path.split("?").collect::<Vec<&str>>()[1].to_string();
        path = path.split("?").collect::<Vec<&str>>()[0].to_string();
    }

    let sha256_token: &str = match headers
        .iter()
        .find(|header: &&(String, String)| header.0 == "Cookie")
    {
        Some(header) => header
            .1
            .split("token=")
            .collect::<Vec<&str>>()
            .last()
            .unwrap(),
        None => "",
    };
    println!("Sha256 token: {}", sha256_token.cyan());
    let auth: bool = auth_token(&sha256_token).await;

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
        "/api/v1/messages" => {
            if auth {
                api_messages().await
            } else {
                return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
            }
        }
        _ => get_404().await,
    };

    contents = check_template(&mut contents, get_userdata(sha256_token).await).await;
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

async fn post_comment(params: Vec<&str>, sha256_token: &str) -> String {
    println!("Params: {:?} Token: {}", params, sha256_token);
    return "HTTP/1.1 501 NOT IMPLEMENTED\r\n\r\n".to_string() + "501 Not Implemented";
}

async fn upload(
    title: &str,
    content: &str,
    image_data: Vec<u8>,
    image_name: String,
    email: &str,
) -> String {
    println!(
        "Title: {} Content: {} Image: {} Email: {}",
        title, content, image_name, email
    );

    let mut file = tokio::fs::File::create(format!("images/{}", image_name))
        .await
        .unwrap();

    // write the image to the file
    file.write_all(&image_data).await.unwrap();

    dbconn()
        .execute(
            "INSERT INTO posts (title, content, image, email) VALUES (?1, ?2, ?3, ?4)",
            &[&title, &content, &image_name.as_str(), &email],
        )
        .unwrap();

    return "HTTP/1.1 200 OK\r\n\r\n".to_string() + "200 OK";
}

async fn post(path: String, headers: Vec<(String, String)>, body: String) -> String {
    let body: String = urlencoding::decode(body.as_str()).unwrap().to_string();
    let params: Vec<&str> = body
        .split("&")
        .map(|param: &str| param.trim_end_matches('\0'))
        .collect();
    let sha256_token: &str = match headers
        .iter()
        .find(|header: &&(String, String)| header.0 == "Cookie")
    {
        Some(header) => header
            .1
            .split("token=")
            .collect::<Vec<&str>>()
            .last()
            .unwrap(),
        None => "",
    };
    println!("Sha256 token: {}", sha256_token.cyan());

    match path.as_str() {
        "/login" => post_login(dbconn(), params).await,
        "/logout" => post_logout(),
        "/comment" => post_comment(params, sha256_token).await,
        _ => return "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string() + "404 Not Found",
    }
}

fn parse_http_request(request: &str) -> (String, String, Vec<(String, String)>, String) {
    let mut lines = request.lines();
    let start_line = lines.next();

    match start_line {
        Some(_) => {}
        None => {
            return (
                "ERROR".to_string(),
                "400 Bad Request".to_string(),
                Vec::new(),
                "".to_string(),
            );
        }
    };
    let start_line = start_line.unwrap();
    let mut parts = start_line.split_whitespace();
    let method = parts.next().unwrap().to_string();
    let path = parts.next().unwrap().to_string();

    let mut headers = Vec::new();
    let mut body = String::new();

    for line in &mut lines {
        if line.is_empty() {
            break;
        }
        let mut header_parts = line.splitn(2, ':');
        let key = header_parts.next().unwrap().trim().to_string();
        let value = header_parts.next().unwrap().trim().to_string();
        headers.push((key, value));
    }

    for line in &mut lines {
        body.push_str(line);
        body.push('\n');
    }

    return (method, path, headers, body);
}

fn find_subslice(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

async fn handle_connection(mut socket: tokio::net::TcpStream) {
    let mut complete_buffer: Vec<u8> = Vec::new();
    println!(
        "\nNew connection from {}",
        socket.peer_addr().unwrap().to_string().red()
    );
    let mut buffer: [u8; 16384] = [0; 16384];
    loop {
        let bytes_read: usize = socket.read(&mut buffer).await.unwrap();
        if bytes_read < 16384 {
            complete_buffer.extend_from_slice(&buffer[..bytes_read]);
            break;
        }
        complete_buffer.extend_from_slice(&buffer);
    }
    let string_buffer: std::string::String = String::from_utf8_lossy(&complete_buffer).to_string();

    // dump the request to a file
    let mut file = tokio::fs::File::create("request.dump").await.unwrap();
    file.write_all(&string_buffer.as_bytes()).await.unwrap();

    let (method, path, headers, body) = parse_http_request(&string_buffer);
    if method == "ERROR" {
        socket
            .write_all("HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string().as_bytes())
            .await
            .unwrap();
        return;
    }

    println!("Method: {}, Path: {}", method.green(), path.yellow());
    println!("Headers: {}", format!("{:?}", headers).red());

    let content_type: String = headers
        .iter()
        .find(|header: &&(String, String)| header.0 == "Content-Type")
        .unwrap_or(&("Content-Type".to_string(), "text/plain".to_string()))
        .1
        .to_string();

    if content_type.contains("multipart/form-data") && path == "/upload" && method == "POST" {
        let sha256_token: &str = match headers
            .iter()
            .find(|header: &&(String, String)| header.0 == "Cookie")
        {
            Some(header) => header
                .1
                .split("token=")
                .collect::<Vec<&str>>()
                .last()
                .unwrap(),
            None => "",
        };
        let decoded: JsonValue = get_userdata(sha256_token).await;

        if sha256_token.is_empty() {
            socket
                .write_all("HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string().as_bytes())
                .await
                .unwrap();
            return;
        } else {
            if decoded["email"].is_null() {
                socket
                    .write_all("HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string().as_bytes())
                    .await
                    .unwrap();
                return;
            }
        }

        let boundary: &str = content_type
            .split("boundary=")
            .collect::<Vec<&str>>()
            .last()
            .unwrap();
        let mut parts: Vec<&str> = body.split(boundary).collect();
        parts.remove(0); // remove the first part which is -- at the beginning
        parts.pop(); // remove the last part which is the boundary with -- at the end

        let mut title: &str = "";
        let mut content: &str = "";
        let mut image_name: String = String::new();
        let mut image_data: Vec<u8> = Vec::new();

        for part in parts {
            let lines: std::str::Lines = part.lines();

            let mut lines: Vec<&str> = lines.collect();
            lines.pop();

            let mut part_headers: Vec<(String, String)> = Vec::new();

            for line in &mut lines {
                let mut parts: Vec<&str> = line.split(": ").collect();
                let key: String = parts.remove(0).to_string();
                let value: String = parts.join(": ").to_string();
                if key.is_empty() && value.is_empty() {
                    continue;
                } else if !key.is_empty() && value.is_empty() {
                    break;
                }
                part_headers.push((key, value));
            }

            // the real image data is in complete_buffer, we need to find the start and end of the image data in the complete_buffer and extract it
            // the image data is between the Content-Type header and the last boundary+--

            // as soon as String::from_utf8_lossy return an ? character, we know that we are at the end of the headers (start of the image data)
            // we need to find the end of the image data which is the last boundary+--
            // Create markers for the start and end of the image data
            let boundary_end = b"\r\n"; // standard CRLF following boundary
            let double_crlf = b"\r\n\r\n"; // marks end of headers

            let mut part_data = Vec::new();

            if let Some(mut start) = find_subslice(&complete_buffer, boundary.as_bytes()) {
                start += boundary.len() + boundary_end.len(); // Skip boundary and CRLF

                while let Some(end) = find_subslice(&complete_buffer[start..], boundary.as_bytes()) {
                    let part_start = start;
                    let part_end = start + end;

                    // Extract headers
                    if let Some(headers_end) =
                        find_subslice(&complete_buffer[part_start..part_end], double_crlf)
                    {
                        let headers_end = part_start + headers_end + double_crlf.len();

                        // Extract headers as string
                        let headers = std::str::from_utf8(
                            &complete_buffer[part_start..headers_end - double_crlf.len()],
                        )
                        .unwrap();

                        // Check if this part is the image part by inspecting the headers
                        if headers.contains("Content-Type: image/") {
                            // Extract content
                            let content_start = headers_end;
                            let content_end = part_end - boundary_end.len();

                            if content_start < content_end {
                                part_data.extend_from_slice(
                                    &complete_buffer[content_start..content_end],
                                );
                            }
                        }
                    }

                    // Move to the start of the next part
                    start = part_end + boundary.len() + boundary_end.len();
                }
            }

            if part_data.len() > 500 {
                println!("PART Data: {}", part_data.len());
            } else {
                println!("PART Data: {:?}", part_data);
            }

            println!("PART Headers: {:?}", part_headers);

            let content_disposition: &str = part_headers
                .iter()
                .find(|header: &&(String, String)| header.0 == "Content-Disposition")
                .unwrap()
                .1
                .as_str();

            let mut name: String = content_disposition
                .split(" name=")
                .collect::<Vec<&str>>()
                .last()
                .unwrap()
                .replace("\"", "");

            name = name
                .split(";")
                .collect::<Vec<&str>>()
                .first()
                .unwrap()
                .to_string();
            let name: &str = name.as_str();

            match name {
                "title" => {
                    title = lines.last().unwrap();
                }
                "content" => {
                    content = lines.last().unwrap();
                }
                "image" => {
                    image_name = content_disposition
                        .split("filename=")
                        .collect::<Vec<&str>>()
                        .last()
                        .unwrap()
                        .replace("\"", "");
                    // concat with random string to avoid overwriting
                    image_name = format!(
                        "{}-{}.{}",
                        image_name
                            .split('.')
                            .collect::<Vec<&str>>()
                            .first()
                            .unwrap(),
                        Uuid::new_v4(),
                        image_name.split('.').collect::<Vec<&str>>().last().unwrap()
                    );
                    image_data = part_data;
                }
                _ => {}
            }
        }

        let email: &str = decoded["email"].as_str().unwrap();
        let response: String = upload(title, content, image_data, image_name, email).await;
        println!("Response: {}", response.cyan());
        socket.try_write(response.as_bytes()).unwrap();
        return;
    }

    if path.contains("images") || path.contains("favicon.ico") {
        let image: &str = path
            .split('/')
            .collect::<Vec<&str>>()
            .last()
            .unwrap()
            .split('?')
            .collect::<Vec<&str>>()
            .first()
            .unwrap();
        let response: String = "HTTP/1.1 200 OK\r\nContent-Type: image/x-icon\r\n\r\n".to_string();
        socket.write_all(response.as_bytes()).await.unwrap();
        let favicon: Vec<u8> = fs::read(format!("images/{}", image)).await.unwrap();
        socket.write_all(&favicon).await.unwrap();
        return;
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
            "CREATE TABLE IF NOT EXISTS posts (
                    posts_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    email TEXT NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT,
                    image VARCHAR(255),
                    datetime DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(email) REFERENCES users(email)
                );",
            [],
        )
        .unwrap();

    dbconn()
        .execute(
            "CREATE TABLE IF NOT EXISTS comments (
                    comment_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    posts_id INTEGER NOT NULL,
                    email TEXT NOT NULL,
                    comment_message TEXT,
                    datetime DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(posts_id) REFERENCES posts(posts_id),
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
        match TcpListener::bind(format!("0.0.0.0:{}", port)).await {
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
