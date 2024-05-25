use colored::Colorize;
use json::JsonValue;
use rusqlite::Connection;
use tokio::fs;

use crate::db::dbconn;
use crate::http::token::auth_token;
use crate::http::token::get_userdata;

async fn check_template(contents: &mut String, userdata: JsonValue) -> String {
    for (key, value) in userdata.entries() {
        let key: String = key.to_string();
        let value: String = value.to_string();
        let key: String = format!("&{{{}}}", key);
        *contents = contents.replace(&key, &value);
    }

    return contents.to_string();
}

async fn api_posts() -> String {
    let mut posts: String = "[".to_string();
    let dbconn: Connection = dbconn();

    let mut stmt = dbconn
        .prepare(
            "SELECT post_id, title, content, email, datetime, image FROM posts ORDER BY datetime DESC",
        )
        .unwrap();
    let posts_iter = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
        ))
    });

    for post in posts_iter.unwrap() {
        let post = post.unwrap();
        let post_id: i64 = post.0;
        let title: String = post.1;
        let content: String = post.2;
        let email: String = post.3;
        let datetime: String = post.4 + " UTC";
        let image: String = post.5;

        let mut stmt = dbconn
            .prepare("SELECT email, content, datetime FROM comments WHERE post_id = ? ORDER BY datetime DESC")
            .unwrap();
        let comments_iter = stmt.query_map([post_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
            ))
        });

        let mut comments: String = "[".to_string();
        for comment in comments_iter.unwrap() {
            let comment = comment.unwrap();
            let email: String = comment.0;
            let content: String = comment.1;
            let datetime: String = comment.2 + " UTC";
            comments.push_str(&format!(
                r#"{{"email":"{}","content":"{}","datetime":"{}"}},"#,
                email, content, datetime
            ));
        }
        if comments.chars().last().unwrap() == ',' {
            comments.pop();
        }
        comments.push_str("]");

        let mut stmt = dbconn
            .prepare(
                "SELECT type, COUNT(type) AS count FROM reactions WHERE post_id = ? GROUP BY type",
            )
            .unwrap();
        let reactions_iter = stmt.query_map([post_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))
        });

        let mut reactions: String = "{".to_string();
        for reaction in reactions_iter.unwrap() {
            let reaction = reaction.unwrap();
            let reaction_type: String = reaction.0;
            let reaction_count: i64 = reaction.1;
            reactions.push_str(&format!(r#""{}":{},"#, reaction_type, reaction_count));
        }
        if reactions.chars().last().unwrap() == ',' {
            reactions.pop();
        }
        reactions.push_str("}");

        posts.push_str(&format!(
            r#"{{"post_id":{},"title":"{}","content":"{}","email":"{}","datetime":"{}","image":"{}","comments":{},"reactions":{}}},"#,
            post_id, title, content, email, datetime, image, comments, reactions
        ));
    }

    if posts.chars().last().unwrap() == ',' {
        posts.pop();
    }
    posts.push_str("]");

    return format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
        posts
    );
}

async fn api_userreaction(post_id: i64, email: String) -> String {
    let dbconn: Connection = dbconn();
    let mut stmt = dbconn
        .prepare("SELECT type FROM reactions WHERE post_id = ? AND email = ?")
        .unwrap();
    let reaction_iter = stmt.query_map(
        [
            &post_id as &dyn rusqlite::types::ToSql,
            &email as &dyn rusqlite::types::ToSql,
        ],
        |row| Ok(row.get::<_, String>(0)?),
    );

    let mut reaction: String = "null".to_string();
    for reaction_type in reaction_iter.unwrap() {
        reaction = reaction_type.unwrap();
    }

    return format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{{\"type\":\"{}\"}}",
        reaction
    );
}

async fn api_comments(post_id: i64) -> String {
    let dbconn: Connection = dbconn();
    let mut stmt = dbconn
        .prepare("SELECT email, content, datetime FROM comments WHERE post_id = ? ORDER BY datetime DESC")
        .unwrap();
    let comments_iter = stmt.query_map([post_id], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
        ))
    });

    let mut comments: String = "[".to_string();
    for comment in comments_iter.unwrap() {
        let comment = comment.unwrap();
        let email: String = comment.0;
        let content: String = comment.1;
        let datetime: String = comment.2 + " UTC";
        comments.push_str(&format!(
            r#"{{"email":"{}","content":"{}","datetime":"{}"}}"#,
            email, content, datetime
        ));
    }
    comments.push_str("]");

    return format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\n\r\n{}",
        comments
    );
}

async fn get_ascii_content(directory: &str, file: &str) -> String {
    let contents: String = fs::read_to_string(format!("public/{}/{}", directory, file))
        .await
        .unwrap();
    return format!("HTTP/1.1 200 OK\r\n\r\n{}", contents);
}

async fn get_utf8_content(directory: &str, file: &str) -> Vec<u8> {
    let contents: Vec<u8> = fs::read(format!("public/{}/{}", directory, file))
        .await
        .unwrap();
    return contents;
}

fn match_type(path: &str) -> (String, String) {
    if path == "/" {
        return ("pages".to_string(), "index.html".to_string());
    }

    if !path.contains(".") {
        if path.contains("api") {
            let file: String = path
                .split("/")
                .collect::<Vec<&str>>()
                .last()
                .unwrap()
                .to_string();
            let directory: String = "api".to_string();
            return (directory, file);
        }
        let file: String = path
            .split("/")
            .collect::<Vec<&str>>()
            .last()
            .unwrap()
            .to_string()
            + ".html";
        let directory: String = "pages".to_string();
        return (directory, file);
    }

    let path: Vec<&str> = path.split("/").collect();
    let mut path: Vec<String> = path.iter().map(|x: &&str| x.to_string()).collect();
    let file: String = path.pop().unwrap();
    let directory: String = path.clone().join("/");
    return (directory, file);
}

async fn match_plain_content(
    requested_endpoint: (String, String),
    sha256_token: &str,
    queries: Vec<(String, String)>,
) -> String {
    let auth: bool = auth_token(&sha256_token).await;
    let decoded: JsonValue = get_userdata(&sha256_token).await;
    let email: String = decoded["email"].to_string();

    if requested_endpoint.0 == "api" {
        let post_id = queries
            .iter()
            .find(|(key, _)| key == "post_id")
            .unwrap_or(&("post_id".to_string(), "1".to_string()))
            .1
            .parse::<i64>()
            .unwrap();
        match requested_endpoint.1.as_str() {
            "posts" => return api_posts().await,
            "comments" => return api_comments(post_id).await,
            "userreaction" => return api_userreaction(post_id, email).await,
            _ => return "HTTP/1.1 404 NOT FOUND\r\nContent-Length: 0\r\n\r\n".to_string(),
        }
    }

    if !file_or_dir_exists(&requested_endpoint.0, &requested_endpoint.1).await {
        return get_ascii_content("pages", "404.html").await;
    }

    if !auth && requested_endpoint.1 != "login.html" {
        return "HTTP/1.1 301 MOVED PERMANENTLY\r\nLocation: /login\r\nContent-Length: 0\r\n\r\n"
            .to_string()
            + "301 Moved Permanently";
    }

    if auth && requested_endpoint.1 == "login.html" {
        return "HTTP/1.1 301 MOVED PERMANENTLY\r\nLocation: /\r\nContent-Length: 0\r\n\r\n"
            .to_string()
            + "301 Moved Permanently";
    }

    return get_ascii_content(&requested_endpoint.0, &requested_endpoint.1).await;
}

async fn file_or_dir_exists(directory: &str, file: &str) -> bool {
    let path: String = format!("public/{}/{}", directory, file);
    let exists: bool = fs::metadata(path).await.is_ok();
    return exists;
}

pub async fn get(mut path: String, headers: Vec<(String, String)>) -> (String, Vec<u8>) {
    let mut queries: Vec<(String, String)> = Vec::new();
    if path.contains("?") {
        let path_clone: String = path.clone();
        let temp: Vec<&str> = path_clone.split("?").collect();
        path = temp[0].to_string();
        let queries_str: &str = temp[1];
        let queries_str: Vec<&str> = queries_str.split("&").collect();
        for query in queries_str {
            let query: Vec<&str> = query.split("=").collect();
            queries.push((query[0].to_string(), query[1].to_string()));
        }
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
    println!("Requested endpoint: {}", requested_endpoint.1.red());

    let mut binary_content: Vec<u8> = Vec::new();
    let _temp: Vec<&str> = requested_endpoint.1.split(".").collect::<Vec<&str>>();
    let extension: &&str = _temp.last().unwrap();

    let mut contents: String;
    if ["png", "jpg", "jpeg", "gif", "ico"].contains(&extension) {
        if &requested_endpoint.0 != "images" {
            requested_endpoint.0 = "images".to_string();
        }
        contents = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: image/{}\r\n\r\n",
            extension
        );
        binary_content = get_utf8_content(&requested_endpoint.0, &requested_endpoint.1).await;
    } else {
        contents = match_plain_content(requested_endpoint.clone(), sha256_token, queries).await;
    }

    contents = check_template(&mut contents, get_userdata(sha256_token).await).await;
    return (contents, binary_content);
}
