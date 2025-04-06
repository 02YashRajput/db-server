mod auth;
mod parser;
mod db;
mod logger; 
use std::env;
mod cleaner;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::db::Db;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let port = args.get(1).map(|s| s.to_string()).unwrap_or("4000".to_string());
    let address = format!("127.0.0.1:{}", port);

    let db: Db = Arc::new(Mutex::new(HashMap::new()));
    cleaner::start_cleaner(db.clone()).await;

    let listener = TcpListener::bind(&address).await?;
    println!("Server listening on {}", address);

    loop {
        let (mut socket, _) = listener.accept().await?;

        let db = db.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            // Wait for exactly one AUTH command
            line.clear();
            let bytes = reader.read_line(&mut line).await.unwrap_or(0);
            if bytes == 0 {
                return; // client disconnected
            }

            let parts: Vec<&str> = line.trim().split_whitespace().collect();
            if parts.len() == 3 && parts[0].eq_ignore_ascii_case("AUTH") {
                let username = parts[1];
                let password = parts[2];

                if auth::authenticate(username, password) {
                    writer.write_all(b"AUTH OK\n").await.unwrap();
                } else {
                    writer.write_all(b"AUTH FAILED\n").await.unwrap();
                    return;
                }
            } else {
                // Invalid first command or bad AUTH format
                writer.write_all(b"Expected AUTH <username> <password>\n").await.unwrap();
                return;
            }

            // =========== AUTH SUCCESS, ENTER COMMAND REPL ==============
            loop {
                line.clear();
                let bytes_read = reader.read_line(&mut line).await.unwrap_or(0);
                if bytes_read == 0 {
                    break;
                }

                let response = parser::parse_statement(line.trim(), &db);
                writer.write_all(format!("{}\n", response).as_bytes()).await.unwrap();
            }
        });
    }
}
