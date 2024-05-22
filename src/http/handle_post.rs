use rusqlite::Connection;
use crate::db::dbconn;
use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use sha256::digest;
use colored::Colorize;

fn post_logout() -> String {
    // delete cookie from browser
    return format!("HTTP/1.1 301 OK\r\nSet-Cookie: token=; Max-Age=0\r\nLocation: /\r\nContent-Length: 0\r\n\r\n");
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
        "HTTP/1.1 301 Moved Permanently\r\nSet-Cookie: token={}; Path=/\r\nLocation: /\r\nContent-Length: 0\r\n\r\n",
        &token
    );
}

async fn post_comment(params: Vec<&str>, sha256_token: &str) -> String {
    println!("Params: {:?} Token: {}", params, sha256_token);
    return "HTTP/1.1 501 NOT IMPLEMENTED\r\n\r\n".to_string() + "501 Not Implemented";
}

pub async fn post(path: String, headers: Vec<(String, String)>, body: String) -> (String, Vec<u8>) {
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

    let content = match path.as_str() {
        "/api/login" => post_login(dbconn(), params).await,
        "/api/logout" => post_logout(),
        "/api/comment" => post_comment(params, sha256_token).await,
        _ => "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string() + "404 Not Found",
    };

    return (content, Vec::new());
}