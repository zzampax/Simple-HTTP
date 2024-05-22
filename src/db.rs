use rusqlite::Connection;

pub fn dbconn() -> Connection {
    return Connection::open("users.db").unwrap();
}

pub fn init_db() {
    dbconn()
        .execute(
            "CREATE TABLE IF NOT EXISTS users (
                    email TEXT PRIMARY KEY,
                    password TEXT NOT NULL
            );",
            [],
        )
        .unwrap();

    dbconn()
        .execute(
            "CREATE TABLE IF NOT EXISTS posts (
                    post_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    email TEXT NOT NULL,
                    title TEXT NOT NULL,
                    content TEXT,
                    image VARCHAR(255),
                    datetime DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(email) REFERENCES users(email) ON DELETE CASCADE
                );",
            [],
        )
        .unwrap();

    dbconn()
        .execute(
            "CREATE TABLE IF NOT EXISTS comments (
                    comment_id INTEGER PRIMARY KEY AUTOINCREMENT,
                    post_id INTEGER NOT NULL,
                    email TEXT NOT NULL,
                    content TEXT,
                    datetime DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(post_id) REFERENCES posts(post_id) ON DELETE CASCADE,
                    FOREIGN KEY(email) REFERENCES users(email) ON DELETE CASCADE
                );",
            [],
        )
        .unwrap();

    dbconn()
        .execute(
            "CREATE TABLE IF NOT EXISTS tokens (
                    email TEXT PRIMARY KEY,
                    token TEXT NOT NULL,
                    timestamp DEFAULT CURRENT_TIMESTAMP,
                    FOREIGN KEY(email) REFERENCES users(email) ON DELETE CASCADE
                );",
            [],
        )
        .unwrap();
}
