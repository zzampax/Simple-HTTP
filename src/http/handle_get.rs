use json::JsonValue;
use tokio::fs;
use colored::Colorize;

use crate::http::token::auth_token;
use crate::http::token::get_userdata;

use crate::http::handle_api::handle_api;

async fn check_template(contents: &mut String, userdata: JsonValue) -> String {
    for (key, value) in userdata.entries() {
        let key: String = key.to_string();
        let value: String = value.to_string();
        let key: String = format!("&{{{}}}", key);
        *contents = contents.replace(&key, &value);
    }

    return contents.to_string();
}

async fn get_404() -> String {
    let contents: String = fs::read_to_string("public/pages/404.html").await.unwrap();
    return format!("HTTP/1.1 404 NOT FOUND\r\n\r\n{}", contents);
}

async fn get_plain_content(directory: &str, file: &str) -> String {
    let contents: String = fs::read_to_string(format!("public/{}/{}", directory, file)).await.unwrap();
    return format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);
}

async fn get_bytes_content(directory: &str, file: &str) -> Vec<u8> {
    println!("{}", format!("public/{}/{}", directory, file).red());
    let contents: Vec<u8> = fs::read(format!("public/{}/{}", directory, file)).await.unwrap();
    return contents;
}

fn match_type(path: &str) -> (String, String) {
    // If path does not have a file extension, search from the directory PAGES
    // else, search from the directory in the path + file extension
    if path == "/" {
        return ("pages".to_string(), "index.html".to_string());
    }

    if !path.contains(".") {
        if path.contains("api") {
            let file: String = path.split("/").collect::<Vec<&str>>().last().unwrap().to_string();
            let directory: String = "api".to_string();
            return (directory, file);
        }
        let file: String = path.split("/").collect::<Vec<&str>>().last().unwrap().to_string() + ".html";
        let directory: String = "pages".to_string();
        return (directory, file);
    }

    // path example: /css/style.css -> search if css directory exists, then if style.css exists
    let path: Vec<&str> = path.split("/").collect();
    let mut path: Vec<String> = path.iter().map(|x: &&str| x.to_string()).collect();
    let file: String = path.pop().unwrap();
    let directory: String = path.clone().join("/");
    // check if directory exists
    return (directory, file);
}

async fn match_plain_content(requested_endpoint: (String, String), sha256_token: &str) -> String {
    let auth: bool = auth_token(&sha256_token).await;
    if requested_endpoint.0 == "api" {
        return handle_api(&requested_endpoint.1).await;
    }

    if !file_or_dir_exists(&requested_endpoint.0, &requested_endpoint.1).await {
        return get_404().await;
    }
    
    if !auth && requested_endpoint.1 != "login.html" {
        return "HTTP/1.1 301 MOVED PERMANENTLY\r\nLocation: /login\r\nContent-Length: 0\r\n\r\n".to_string() + "301 Moved Permanently";
    }

    if auth && requested_endpoint.1 == "login.html" {
        return "HTTP/1.1 301 MOVED PERMANENTLY\r\nLocation: /\r\nContent-Length: 0\r\n\r\n".to_string() + "301 Moved Permanently";
    }

    return get_plain_content(&requested_endpoint.0, &requested_endpoint.1).await;
}

async fn file_or_dir_exists(directory: &str, file: &str) -> bool {
    let path: String = format!("public/{}/{}", directory, file);
    let exists: bool = fs::metadata(path).await.is_ok();
    return exists;
}

pub async fn get(mut path: String, headers: Vec<(String, String)>) -> (String, Vec<u8>) {
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

    let mut requested_endpoint: (String, String) = match_type(&path);
    println!("Requested endpoint: {:?}", requested_endpoint);
    let mut contents: String = match_plain_content(requested_endpoint.clone(), sha256_token).await;

    // match extension of file
    let mut binary_content: Vec<u8> = Vec::new();

    let _temp: Vec<&str> = requested_endpoint.1.split(".").collect::<Vec<&str>>();
    let extension: &&str = _temp.last().unwrap();

    if ["png", "jpg", "jpeg", "gif", "ico"].contains(&extension) {
        if &requested_endpoint.0 != "images" {
            requested_endpoint.0 = "images".to_string();
        }
        contents = format!("HTTP/1.1 200 OK\r\nContent-Type: image/{}\r\n\r\n", extension);
        binary_content = get_bytes_content(&requested_endpoint.0, &requested_endpoint.1).await;
    }
    contents = check_template(&mut contents, get_userdata(sha256_token).await).await;
    
    return (contents, binary_content);
}