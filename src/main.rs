use tokio::net::TcpListener;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use std::fs;

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

    let contents: String = fs::read_to_string(format!("pages/{}", path)).unwrap_or_else(|_| {
        path = "404.html".to_string();
        fs::read_to_string(format!("pages/{}", path)).unwrap()
    });

    let response: String = format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);

    println!("Sending: {}", path);
    return response;
}

async fn post(path: String, headers: Vec<&str>, body: &str) -> String{
    println!("Body of request: {}", body);

    return get(path, headers).await;
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
    
    let (method, path, _version) = (request_line.next().unwrap().to_string(), request_line.next().unwrap().to_string(), request_line.next().unwrap().to_string());    
    let headers: Vec<&str> = lines.clone().take_while(|line| !line.is_empty()).collect();

    let body = lines.last().unwrap();

    println!("Method: {}, Path: {}", method, path);

    if let Some(accept_header) = headers.iter().find(|&header| header.starts_with("Accept: ")) {
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
    let mut port = 3000;
    loop {
        match TcpListener::bind(format!("127.0.0.1:{}", port)).await {
            Ok(listener) => {
                println!("\n --> Server running on port {}! <--", port);

                loop {
                    let (socket, _) = listener.accept().await.unwrap();
                    tokio::spawn(handle_connection(socket));
                }
            },
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