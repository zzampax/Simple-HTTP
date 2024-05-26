mod db;
mod http;
mod multipart;

use colored::Colorize;
use http::{handle_get, handle_post};
use std::str;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use uuid::Uuid;

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

async fn _dump_request(buffer: &Vec<u8>) {
    let mut file = tokio::fs::File::create(format!("dumps/request-{}.dump", Uuid::new_v4()))
        .await
        .unwrap();
    file.write_all(buffer).await.unwrap();
}

async fn handle_buffer_data(stream: &mut tokio::net::TcpStream) -> std::io::Result<Vec<u8>> {
    let mut reader = tokio::io::BufReader::new(stream);
    let mut request = Vec::new();
    let mut headers = Vec::new();
    let mut content_length = 0;

    // Read the headers
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line == "\r\n" {
            break;
        }
        if line.starts_with("Content-Length:") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(length_str) = parts.get(1) {
                content_length = length_str.parse::<usize>().unwrap_or(0);
            }
        }
        headers.push(line.clone());
        request.extend_from_slice(line.as_bytes());
    }

    // Read the body if there's a Content-Length
    if content_length > 0 {
        let mut body: Vec<u8> = vec![0; content_length];
        reader.read_exact(&mut body).await?;
        request.extend_from_slice("\r\n".as_bytes());
        request.extend_from_slice(&body);
    }

    Ok(request)
}

async fn handle_connection(mut socket: tokio::net::TcpStream) {
    println!(
        "\nNew connection from {}",
        socket.peer_addr().unwrap().to_string().red()
    );

    // BUFFERING
    let complete_buffer: Vec<u8> = match handle_buffer_data(&mut socket).await {
        Ok(buffer) => buffer,
        Err(_) => {
            println!("Error reading from socket");
            return;
        }
    };
    let string_buffer: std::string::String = String::from_utf8_lossy(&complete_buffer).to_string();

    // _dump_request(&complete_buffer).await;

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

    if response.1.len() > 0 {
        socket.write_all(&response.1).await.unwrap();
    }
}

#[tokio::main]
async fn main() {
    db::init_db();

    let ports: Vec<u16> = vec![80, 8000, 8080, 8888];
    let mut port_index: usize = 0;
    loop {
        match TcpListener::bind(format!("0.0.0.0:{}", ports[port_index])).await {
            Ok(listener) => {
                println!("\n --> Server running on port {}! <--", ports[port_index]);

                loop {
                    let (socket, _) = listener.accept().await.unwrap();
                    tokio::spawn(handle_connection(socket));
                }
            }
            Err(_) => {
                println!("Port {} is in use, trying next port...", ports[port_index]);
                if ports[port_index] == *ports.last().unwrap(){
                    println!("All safe ports are in use, exiting...");
                    break;
                }
                port_index += 1;
            }
        }
    }
}
