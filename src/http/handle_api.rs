use crate::db::dbconn;
use rusqlite::Connection;

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

pub async fn handle_api(endpoint: &str) -> String {
    match endpoint {
        "messages" => api_messages().await,
        _ => "HTTP/1.1 404 NOT FOUND\r\n\r\n".to_string(),
    }
}