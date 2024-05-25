use base64::{engine::general_purpose::URL_SAFE, Engine as _};
use colored::Colorize;
use json::JsonValue;
use rusqlite::Connection;
use sha256::digest;

use crate::db::dbconn;
use crate::http::token;

fn post_logout() -> String {
    return format!("HTTP/1.1 301 OK\r\nSet-Cookie: token=; Max-Age=0; Path=/\r\nLocation: /\r\nContent-Length: 0\r\n\r\n");
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
    let mut vec_params: Vec<(String, String)> = Vec::new();
    for param in params {
        let key_value: Vec<&str> = param.split('=').collect();
        if key_value.len() == 2 {
            let key: String = key_value[0].to_string();
            let mut value: String = key_value[1].to_string();
            value = value.replace("+", " ");
            vec_params.push((key, value));
        }
    }
    let decoded: JsonValue = token::get_userdata(sha256_token).await;
    if decoded == JsonValue::Null {
        return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
    }
    let email: &str = decoded["email"].as_str().unwrap();

    let content: &str = match vec_params.iter().find(|param| param.0 == "content") {
        Some(param) => param.1.as_str(),
        None => return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request",
    };

    if content.trim_end_matches('\n').is_empty() {
        return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request";
    }
    if content.is_empty() {
        return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request";
    }

    let content: String = urlencoding::encode(content).to_string();
    let post_id: i64 = match vec_params.iter().find(|param| param.0 == "post_id") {
        Some(param) => param.1.parse::<i64>().unwrap(),
        None => return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request",
    };
    let dbconn: Connection = dbconn();
    dbconn
        .execute(
            "INSERT INTO comments (email, content, post_id) VALUES (?1, ?2, ?3)",
            &[
                &email as &dyn rusqlite::ToSql,
                &content as &dyn rusqlite::ToSql,
                &post_id as &dyn rusqlite::ToSql,
            ],
        )
        .unwrap();

    return "HTTP/1.1 301 Moved Permanently\r\nLocation: /\r\nContent-Length: 0\r\n\r\n"
        .to_string();
}

async fn post_reaction(params: Vec<&str>, sha256_token: &str) -> String {
    println!("Params: {:?}, Token: {}", params, sha256_token);
    let decoded: JsonValue = token::get_userdata(sha256_token).await;
    if decoded == JsonValue::Null {
        return "HTTP/1.1 401 UNAUTHORIZED\r\n\r\n".to_string() + "401 Unauthorized";
    }
    let email: &str = decoded["email"].as_str().unwrap();
    let dbconn: Connection = dbconn();
    let post_id: i64 = match params.iter().find(|param| param.starts_with("post_id=")) {
        Some(param) => param.split('=').collect::<Vec<&str>>()[1]
            .trim_end_matches('\n')
            .parse::<i64>()
            .unwrap(),
        None => return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request",
    };
    let reaction: &str = match params.iter().find(|param| param.starts_with("reaction=")) {
        Some(param) => param.split('=').collect::<Vec<&str>>()[1],
        None => return "HTTP/1.1 400 BAD REQUEST\r\n\r\n".to_string() + "400 Bad Request",
    };
    let reaction: &str = reaction.trim_end_matches('\n');
    let reaction_exists: bool = dbconn
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM reactions WHERE email = ?1 AND post_id = ?2)",
            &[
                &email as &dyn rusqlite::ToSql,
                &post_id as &dyn rusqlite::ToSql,
            ],
            |row| row.get(0),
        )
        .unwrap();

    if reaction_exists {
        dbconn
            .execute(
                "UPDATE reactions SET type = ?1 WHERE email = ?2 AND post_id = ?3",
                &[
                    &reaction as &dyn rusqlite::ToSql,
                    &email as &dyn rusqlite::ToSql,
                    &post_id as &dyn rusqlite::ToSql,
                ],
            )
            .unwrap();
        return "HTTP/1.1 200 OK\r\n\r\n {\"status\": \"ok\"}".to_string();
    }

    dbconn
        .execute(
            "INSERT INTO reactions (email, post_id, type) VALUES (?1, ?2, ?3)",
            &[
                &email as &dyn rusqlite::ToSql,
                &post_id as &dyn rusqlite::ToSql,
                &reaction as &dyn rusqlite::ToSql,
            ],
        )
        .unwrap();

    return "HTTP/1.1 200 OK\r\n\r\n {\"status\": \"ok\"}".to_string();
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
        "/api/reaction" => post_reaction(params, sha256_token).await,
        _ => "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string() + "404 Not Found",
    };

    return (content, Vec::new());
}
