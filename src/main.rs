use std::fs;
use sqlite::{Connection, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use sha256::digest;
use json;
use urlencoding;

async fn get(mut path: String, _headers: Vec<&str>) -> String {
    if path == "/" {
        path = "index.html".to_string();
    }

    let mut query_string = "".to_string();
    //check for query string
    if path.contains("?") {
        query_string = path.split("?").collect::<Vec<&str>>()[1].to_string();
        path = path.split("?").collect::<Vec<&str>>()[0].to_string();
    }

    if query_string != "" {
        println!("Query String: {}", query_string);
    }

    if !path.contains(".") {
        path = format!("{}.html", path);
    }

    println!("Path: {}", path);

    if path != "/login.html" {
        let mut token = "";
        for header in _headers {
            if header.starts_with("Cookie: token=") {
                token = header.split("token=").collect::<Vec<&str>>()[1];
            }
        }

        if token.is_empty() {
            return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
        }
    }

    let contents: String = fs::read_to_string(format!("pages/{}", path)).unwrap_or_else(|_| {
        path = "404.html".to_string();
        fs::read_to_string(format!("pages/{}", path)).unwrap()
    });

    let response: String = format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);

    println!("Sending: {}", path);
    return response;
}

async fn post(path: String, _headers: Vec<&str>, body: &str) -> String {
    println!("Body of request: {}", body);

    let body: String = urlencoding::decode(body).unwrap().to_string();
    let params: Vec<&str> = body.split("&").map(|param: &str| param.trim_end_matches('\0')).collect();

    println!("Path: {}", path);

    match path.as_str() {
        "/login" => {
            let mut email = "";
            let mut password = "";
            for param in params {
                let key_value: Vec<&str> = param.split('=').collect();
                if key_value.len() == 2 {
                    let key = key_value[0].trim();
                    let value = key_value[1].trim();
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

            let to_encode = json::object! {
                user: email,
                pass: password
            };

            let token = digest(URL_SAFE.encode(&to_encode.dump().as_bytes()));

            

            // ENCRYPT THE TOKEN WITH AES
            
            return format!(
                "HTTP/1.1 301 OK\r\nSet-Cookie: token={}\r\nLocation: /\r\nContent-Length: 0\r\n\r\n",
                token
            );
        }
        "/message" => {
            // check for cookie
            let mut token = "";
            for header in _headers {
                if header.starts_with("Cookie: token=") {
                    token = header.split("token=").collect::<Vec<&str>>()[1];
                }
            }

            if token.is_empty() {
                return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
            }

            let decoded = URL_SAFE.decode(token.as_bytes()).unwrap();
            let decoded = String::from_utf8(decoded).unwrap();
            let decoded: json::JsonValue = json::parse(&decoded).unwrap();

            let mut message = "";
            for param in params {
                let key_value: Vec<&str> = param.split("=").collect();
                match key_value[0] {
                    "message" => message = key_value[1],
                    _ => (),
                }
            }

            if message.is_empty() {
                return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request";
            }

            println!("{} says: {}", decoded["user"], message);

            // response will have a Location header and a redirect to the home page
            return format!("HTTP/1.1 301 OK\r\nLocation: /\r\nContent-Length: 0\r\n\r\n");
        }
        _ => return "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string() + "404 Not Found",
    }
}

async fn handle_connection(mut socket: tokio::net::TcpStream) {
    let mut buffer = [0; 16384];
    socket.read(&mut buffer).await.unwrap();
    println!("\nNew connection from {}", socket.peer_addr().unwrap());

    let string_buffer = String::from_utf8_lossy(&buffer);
    let mut lines = string_buffer.lines();

    let request_line = lines.next().unwrap();
    // split the request line into three variables
    let mut request_line = request_line.split_whitespace();

    let (method, path, _version) = (
        request_line.next().unwrap().to_string(),
        request_line.next().unwrap().to_string(),
        request_line.next().unwrap().to_string(),
    );
    let headers: Vec<&str> = lines.clone().take_while(|line| !line.is_empty()).collect();

    let body = lines.last().unwrap();

    println!("Method: {}, Path: {}", method, path);

    if let Some(accept_header) = headers
        .iter()
        .find(|&header| header.starts_with("Accept: "))
    {
        if accept_header.contains("image") && path.contains("favicon.ico") {
            println!("Sending: /favicon.ico");
            let response = "HTTP/1.1 200 OK\r\nContent-Type: image/x-icon\r\n\r\n".to_string();
            socket.write_all(response.as_bytes()).await.unwrap();
            let favicon = fs::read("pages/favicon.ico").unwrap();
            socket.write_all(&favicon).await.unwrap();
            return;
        }
    }

    let response = match method.as_str() {
        "GET" => get(path, headers).await,
        "POST" => post(path, headers, body).await,
        _ => "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string() + "404 Not Found",
    };

    socket.try_write(response.as_bytes()).unwrap();
}

#[tokio::main]
async fn main() {
    let dbconn = Connection::open("users.db").unwrap();
    dbconn.execute("CREATE TABLE IF NOT EXISTS users (sha256 TEXT, token TEXT)").unwrap();

    let mut port = 3000;
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
