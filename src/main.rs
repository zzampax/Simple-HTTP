mod db;
mod http;
mod multipart;

use colored::Colorize;
use http::{handle_get, handle_post};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

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

async fn handle_connection(mut socket: tokio::net::TcpStream) {
    // BUFFERING
    // put the whole request in a buffer (Vec<u8>)
    // The buffer is filled with 16KB chunks until the whole request is read
    // The buffer is then converted to a string and parsed
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
    //let mut file = tokio::fs::File::create("request.dump").await.unwrap();
    //file.write_all(&string_buffer.as_bytes()).await.unwrap();

    let (method, path, headers, body) = parse_http_request(&string_buffer);
    if method == "ERROR" {
        socket
            .write_all("HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string().as_bytes())
            .await
            .unwrap();
        return;
    }

    println!("Method: {}, Path: {}", method.green(), path.yellow());
    println!("Headers: ");
    for header in &headers {
        println!("-> {}: {}", header.0.blue(), header.1.blue());
    }

    let content_type: String = headers
        .iter()
        .find(|header: &&(String, String)| header.0 == "Content-Type")
        .unwrap_or(&("Content-Type".to_string(), "text/plain".to_string()))
        .1
        .to_string();

    if content_type.contains("multipart/form-data") && path == "/api/upload" && method == "POST" {
        let response: String = multipart::upload(headers, &body, complete_buffer).await;
        socket.try_write(response.as_bytes()).unwrap();
        return;
    }

    let response: (String, Vec<u8>) = match method.as_str() {
        "GET" => handle_get::get(path, headers).await,
        "POST" => handle_post::post(path, headers, body).await,
        _ => (
            "HTTP/1.1 405 METHOD NOT ALLOWED\r\n\r\n".to_string(),
            Vec::new(),
        ),
    };

    socket.write_all(response.0.as_bytes()).await.unwrap();
    // check if the response has Vec<u8> content
    if response.1.len() > 0 {
        socket.write_all(&response.1).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    db::init_db();

    let mut port: i32 = 8080;
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
