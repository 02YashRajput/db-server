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

pub type DbMap = Arc<Mutex<HashMap<String, db::DbInstance>>>;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let port = args.get(1).map(|s| s.to_string()).unwrap_or("4000".to_string());
    let address = format!("127.0.0.1:{}", port);

    let all_dbs: DbMap = Arc::new(Mutex::new(HashMap::new()));

    let listener = TcpListener::bind(&address).await?;
    println!("Server running on {}", address);

    loop {
        let (mut socket, _) = listener.accept().await?;
        let all_dbs = all_dbs.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut current_db: Option<Arc<Mutex<HashMap<String, db::ValueWithExpiry>>>> = None;
            let mut line = String::new();

            loop {
                line.clear();
                if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                    break;
                }

                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "create" if parts.len() == 2 => if current_db.is_some() {
                        writer.write_all(b"Cannot create a database. A database is already selected.\n").await.unwrap();
                    } else {
                        let db_name = parts[1].to_string();
                        
                        writer.write_all(b"Do you want authentication (yes/no)?\n").await.unwrap();
                        let mut auth_line = String::new();
                        reader.read_line(&mut auth_line).await.unwrap();
                        let auth_option = auth_line.trim().to_lowercase() == "yes";
                        
                        let db_instance = if auth_option {
                            writer.write_all(b"Enter username:\n").await.unwrap();
                            let mut username_line = String::new();
                            reader.read_line(&mut username_line).await.unwrap();
                            let username = username_line.trim().to_string();
                            
                            writer.write_all(b"Enter password:\n").await.unwrap();
                            let mut password_line = String::new();
                            reader.read_line(&mut password_line).await.unwrap();
                            let password = password_line.trim().to_string();
                            
                            db::DbInstance::new(true, Some(username), Some(password))
                        } else {
                            db::DbInstance::new(false, None, None)
                        };

                       
                        {
                            let mut dbs = all_dbs.lock().unwrap();
                            dbs.insert(db_name, db_instance);
                        }
                        
                        writer.write_all(b"Database created successfully\n").await.unwrap();
                    }
                    "use" if parts.len() == 2 => if current_db.is_some() {
                        writer.write_all(b"Cannot use a different database. A database is already selected.\n").await.unwrap();
                    } else {
                        let db_name = parts[1];
                        let db_instance = {
                            let dbs = all_dbs.lock().unwrap();
                            dbs.get(db_name).cloned()
                        };
                        
                        match db_instance {
                            Some(db_instance) => {
                                if db_instance.require_auth {
                                    let mut authenticated = false;
                                    
                                    while !authenticated {
                                        writer.write_all(b"Username:\n").await.unwrap();
                                        let mut username_line = String::new();
                                        reader.read_line(&mut username_line).await.unwrap();
                                        let username = username_line.trim();
                                        
                                        writer.write_all(b"Password:\n").await.unwrap();
                                        let mut password_line = String::new();
                                        reader.read_line(&mut password_line).await.unwrap();
                                        let password = password_line.trim();
                                        
                                        if db_instance.username.as_deref() == Some(username) && 
                                           db_instance.password.as_deref() == Some(password) {
                                            authenticated = true;
                                            current_db = Some(db_instance.data.clone());
                                            writer.write_all(b"Authentication successful\n").await.unwrap();
                                        } else {
                                            writer.write_all(b"Authentication failed. Try again.\n").await.unwrap();
                                        }
                                    }
                                } else {
                                    current_db = Some(db_instance.data.clone());
                                    writer.write_all(format!("Using database '{}'\n", db_name).as_bytes()).await.unwrap();
                                }
                            }
                            None => {
                                writer.write_all(format!("Database '{}' not found\n", db_name).as_bytes()).await.unwrap();
                            }
                        }
                    }
                    _ => {
                        match &current_db {
                            Some(db) => {
                                let response = parser::parse_statement(line.trim(), &current_db);
                                writer.write_all(format!("{}\n", response).as_bytes()).await.unwrap();
                            }
                            None => {
                                writer.write_all(b"No database selected. Use 'use <dbname>' first.\n").await.unwrap();
                            }
                        }
                    }
                }
            }
        });
    }
}