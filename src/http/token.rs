use json::JsonValue;
use rusqlite::Connection;

use crate::db::dbconn;

pub async fn get_userdata(token: &str) -> JsonValue {
    let dbconn: Connection = dbconn();

    let dbuser: String = match dbconn.query_row(
        "SELECT email FROM tokens WHERE token = ?1",
        [token],
        |row| row.get(0),
    ) {
        Ok(user) => user,
        Err(_) => "".to_string(),
    };

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

pub async fn auth_token(token: &str) -> bool {
    let user_exists: bool = get_userdata(token).await != json::parse("{}").unwrap();
    return user_exists;
}
