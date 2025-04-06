mod auth;
mod parser;
mod db;
mod logger;
mod cleaner;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::env;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

type DbMap = Arc<Mutex<HashMap<String, db::DbInstance>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let port = args.get(1).map(|s| s.to_string()).unwrap_or("4000".to_string());
    let address = format!("127.0.0.1:{}", port);

    let all_dbs: DbMap = Arc::new(Mutex::new(HashMap::new()));

    let listener = TcpListener::bind(&address).await?;
    log_info(&format!("Listening on: {}", address));

    loop {
        let (mut socket, _) = listener.accept().await?;
        let all_dbs_clone = all_dbs.clone();

        tokio::spawn(async move {
            
        })
    }
}