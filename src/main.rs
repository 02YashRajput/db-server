
mod parser;
mod db;
mod logger;
mod cleaner;

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::env;
use tokio::net::TcpListener;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use crate::db::DbMap;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = env::args().collect();
    let port = args.get(1).map(|s| s.to_string()).unwrap_or("4000".to_string());
    let address = format!("127.0.0.1:{}", port);

    let all_dbs: DbMap = Arc::new(Mutex::new(HashMap::new()));
    cleaner::start_cleaner(all_dbs.clone()).await;
    let listener = TcpListener::bind(&address).await?;
    println!("Server running on {}", address);

    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
                continue;
            }
        };
        let all_dbs = all_dbs.clone();

        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut current_db: Option<Arc<Mutex<HashMap<String, db::ValueWithExpiry>>>> = None;
            let mut line = String::new();

            loop {
                line.clear();
                let bytes_read = match reader.read_line(&mut line).await {
                    Ok(0) => break, // Connection closed by client
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Error reading from socket: {}", e);
                        break;
                    }
                };

                if bytes_read == 0 {
                    break;
                }

                let parts: Vec<&str> = line.trim().split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "create" if parts.len() == 2 => {
                        if current_db.is_some() {
                            if let Err(e) = writer.write_all(b"Cannot create a database. A database is already selected.\n").await {
                                eprintln!("Error writing to socket: {}", e);
                                break;
                            }
                        } else {
                            let db_name = parts[1].to_string();
                            
                            if let Err(e) = writer.write_all(b"Do you want authentication (yes/no)?\n").await {
                                eprintln!("Error writing to socket: {}", e);
                                break;
                            }
                            
                            let mut auth_line = String::new();
                            if let Err(e) = reader.read_line(&mut auth_line).await {
                                eprintln!("Error reading auth option: {}", e);
                                break;
                            }
                            
                            let auth_option = auth_line.trim().to_lowercase() == "yes";
                            
                            let db_instance = if auth_option {
                                if let Err(e) = writer.write_all(b"Enter username:\n").await {
                                    eprintln!("Error writing to socket: {}", e);
                                    break;
                                }
                                
                                let mut username_line = String::new();
                                if let Err(e) = reader.read_line(&mut username_line).await {
                                    eprintln!("Error reading username: {}", e);
                                    break;
                                }
                                let username = username_line.trim().to_string();
                                
                                if let Err(e) = writer.write_all(b"Enter password:\n").await {
                                    eprintln!("Error writing to socket: {}", e);
                                    break;
                                }
                                
                                let mut password_line = String::new();
                                if let Err(e) = reader.read_line(&mut password_line).await {
                                    eprintln!("Error reading password: {}", e);
                                    break;
                                }
                                let password = password_line.trim().to_string();
                                
                                db::DbInstance::new(true, Some(username), Some(password))
                            } else {
                                db::DbInstance::new(false, None, None)
                            };

                            {
                                let mut dbs = all_dbs.lock().unwrap();
                                dbs.insert(db_name, db_instance);
                            }
                            
                            if let Err(e) = writer.write_all(b"Database created successfully\n").await {
                                eprintln!("Error writing to socket: {}", e);
                                break;
                            }
                        }
                    }
                    "use" if parts.len() == 2 => {
                        if current_db.is_some() {
                            if let Err(e) = writer.write_all(b"Cannot use a different database. A database is already selected.\n").await {
                                eprintln!("Error writing to socket: {}", e);
                                break;
                            }
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
                                        let mut auth_attempts = 0;
                                        const MAX_AUTH_ATTEMPTS: u8 = 3;
                                        
                                        while !authenticated && auth_attempts < MAX_AUTH_ATTEMPTS {
                                            auth_attempts += 1;
                                            
                                            if let Err(e) = writer.write_all(b"Username:\n").await {
                                                eprintln!("Error writing to socket: {}", e);
                                                break;
                                            }
                                            
                                            let mut username_line = String::new();
                                            if let Err(e) = reader.read_line(&mut username_line).await {
                                                eprintln!("Error reading username: {}", e);
                                                break;
                                            }
                                            let username = username_line.trim();
                                            
                                            if let Err(e) = writer.write_all(b"Password:\n").await {
                                                eprintln!("Error writing to socket: {}", e);
                                                break;
                                            }
                                            
                                            let mut password_line = String::new();
                                            if let Err(e) = reader.read_line(&mut password_line).await {
                                                eprintln!("Error reading password: {}", e);
                                                break;
                                            }
                                            let password = password_line.trim();
                                            
                                            if db_instance.username.as_deref() == Some(username) && 
                                               db_instance.password.as_deref() == Some(password) {
                                                authenticated = true;
                                                current_db = Some(db_instance.data.clone());
                                                if let Err(e) = writer.write_all(format!("Authentication successful Using database '{}'\n", db_name).as_bytes()).await {
                                                    eprintln!("Error writing to socket: {}", e);
                                                    break;
                                                }
                                            } else {
                                                if let Err(e) = writer.write_all(b"Authentication failed. Try again.\n").await {
                                                    eprintln!("Error writing to socket: {}", e);
                                                    break;
                                                }
                                            }
                                        }
                                        
                                        if !authenticated && auth_attempts >= MAX_AUTH_ATTEMPTS {
                                            if let Err(e) = writer.write_all(b"Too many failed authentication attempts. Disconnecting.\n").await {
                                                eprintln!("Error writing to socket: {}", e);
                                            }
                                            break;
                                        }
                                    } else {
                                        current_db = Some(db_instance.data.clone());
                                        if let Err(e) = writer.write_all(format!("Using database '{}'\n", db_name).as_bytes()).await {
                                            eprintln!("Error writing to socket: {}", e);
                                            break;
                                        }
                                    }
                                }
                                None => {
                                    if let Err(e) = writer.write_all(format!("Database '{}' not found\n", db_name).as_bytes()).await {
                                        eprintln!("Error writing to socket: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        match &current_db {
                            Some(_db) => {
                                let response = parser::parse_statement(line.trim(), &current_db);
                                if let Err(e) = writer.write_all(format!("{}\n", response).as_bytes()).await {
                                    eprintln!("Error writing to socket: {}", e);
                                    break;
                                }
                            }
                            None => {
                                if let Err(e) = writer.write_all(b"Unknown command.\n").await {
                                    eprintln!("Error writing to socket: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                }
            }
        });
    }
}