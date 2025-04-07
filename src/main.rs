// =======================================================
// 🧠 INFO: Main Imports and Module Declarations
// =======================================================
mod cleaner;
mod db;
mod logger;
mod parser;
use bcrypt::{hash, verify, DEFAULT_COST};
use crate::db::DbMap;
use db::DbInstance;
use std::collections::HashMap;
use std::env;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use crate::logger::log_info;
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Parse port from args or default to 4000
    let args: Vec<String> = env::args().collect();
    let port = args
        .get(1)
        .map(|s| s.to_string())
        .unwrap_or("4000".to_string());
    let address = format!("0.0.0.0:{}", port);

    // Shared state for all databases
    let all_dbs: DbMap = Arc::new(Mutex::new(HashMap::new()));

    // Start cleaner thread
    cleaner::start_cleaner(all_dbs.clone()).await;

    // Create TCP listener
    let listener = TcpListener::bind(&address).await?;
    log_info(&format!("Server running on {}", address));

    // =======================================================
    // 🧠 INFO: Main Connection Handling Loop
    // =======================================================
    loop {
        let (mut socket, _) = match listener.accept().await {
            Ok(result) => result,
            Err(e) => {
                eprintln!("Error accepting connection: {}", e);
                continue;
            }
        };
        let all_dbs = all_dbs.clone();
        // Spawn new task for each connection
        tokio::spawn(async move {
            let (reader, mut writer) = socket.split();
            let mut reader = BufReader::new(reader);
            let mut line = String::new();
            let mut current_db_instance: Option<Arc<DbInstance>> = None;
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
                    // Create a new database
                    "create" if parts.len() == 2 => {
                        // Check if a database is already selected
                        if current_db_instance.is_some() {
                            if let Err(e) = writer
                                .write_all(
                                    b"Cannot create a database. A database is already selected.\n",
                                )
                                .await
                            {
                                eprintln!("Error writing to socket: {}", e);
                                break;
                            }
                        } else {
                            let db_name = parts[1].to_string();
                            let path = format!("dbs/{}.json", db_name);
                            if Path::new(&path).exists() {
                                if let Err(e) = writer
                                    .write_all(
                                        format!("Error: Database '{}' already exists\n", db_name)
                                            .as_bytes(),
                                    )
                                    .await
                                {
                                    eprintln!("Error writing to socket: {}", e);
                                    break;
                                }
                                continue;
                            }
                            // Ask for authentication preference
                            if let Err(e) = writer
                                .write_all(b"Do you want authentication (yes/no)?\n")
                                .await
                            {
                                eprintln!("Error writing to socket: {}", e);
                                break;
                            }
                            // Read authentication preference
                            let mut auth_line = String::new();
                            if let Err(e) = reader.read_line(&mut auth_line).await {
                                eprintln!("Error reading auth option: {}", e);
                                break;
                            }
                            let auth_option = auth_line.trim().to_lowercase() == "yes";
                            // If authentication is required, ask for username and password
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
                                let hashed_password = match hash(&password, DEFAULT_COST) {
                                    Ok(hashed) => hashed,
                                    Err(e) => {
                                        eprintln!("Error hashing password: {}", e);
                                        if let Err(e) = writer.write_all(b"Error creating database\n").await {
                                            eprintln!("Error writing to socket: {}", e);
                                        }
                                        break;
                                    }
                                };
                                db::DbInstance::new(
                                    db_name.clone(),
                                    true,
                                    Some(username),
                                    Some(hashed_password),
                                )
                            } else {
                                db::DbInstance::new(db_name.clone(), false, None, None)
                            };

                            // Insert new database into shared state
                            {
                                let mut dbs = all_dbs.lock().unwrap();
                                dbs.insert(db_name, db_instance);
                            }

                            // Confirm database creation
                            if let Err(e) =
                                writer.write_all(b"Database created successfully\n").await
                            {
                                eprintln!("Error writing to socket: {}", e);
                                break;
                            }
                        }
                    }
                    // Use a database
                    "use" if parts.len() == 2 => {
                        // Check if a database is already selected
                        if current_db_instance.is_some() {
                            if let Err(e) = writer.write_all(b"Cannot use a different database. A database is already selected.\n").await {
                                eprintln!("Error writing to socket: {}", e);
                                break;
                            }
                        } else {
                            let db_name = parts[1];
                            let db_instance = {
                                let mut dbs = all_dbs.lock().unwrap();

                                // Try to get from memory first
                                if let Some(db) = dbs.get(db_name) {
                                    Some(db.clone())
                                } else {
                                    // If not in memory, try to load from file
                                    if let Some(db) = db::DbInstance::load_from_file(db_name) {
                                        let db_clone = db.clone();
                                        dbs.insert(db_name.to_string(), db);
                                        Some(db_clone)
                                    } else {
                                        None
                                    }
                                }
                            };

                            match db_instance {
                                Some(db_instance) => {
                                    if db_instance.require_auth {
                                        // Ask for authentication
                                        let mut authenticated = false;
                                        let mut auth_attempts = 0;
                                        const MAX_AUTH_ATTEMPTS: u8 = 3; // 3 attempts max

                                        while !authenticated && auth_attempts < MAX_AUTH_ATTEMPTS {
                                            auth_attempts += 1;

                                            if let Err(e) = writer.write_all(b"Username:\n").await {
                                                eprintln!("Error writing to socket: {}", e);
                                                break;
                                            }

                                            let mut username_line = String::new();
                                            if let Err(e) =
                                                reader.read_line(&mut username_line).await
                                            {
                                                eprintln!("Error reading username: {}", e);
                                                break;
                                            }
                                            let username = username_line.trim();

                                            if let Err(e) = writer.write_all(b"Password:\n").await {
                                                eprintln!("Error writing to socket: {}", e);
                                                break;
                                            }

                                            let mut password_line = String::new();
                                            if let Err(e) =
                                                reader.read_line(&mut password_line).await
                                            {
                                                eprintln!("Error reading password: {}", e);
                                                break;
                                            }
                                            let password = password_line.trim();
                                            let is_valid = match verify(password, db_instance.password.as_deref().unwrap_or("")) {
                                                Ok(valid) => valid,
                                                Err(e) => {
                                                    eprintln!("Error verifying password: {}", e);
                                                    if let Err(e) = writer.write_all(b"Authentication error.\n").await {
                                                        eprintln!("Error writing to socket: {}", e);
                                                    }
                                                    break;
                                                }
                                            };
                                            
                                            if db_instance.username.as_deref() == Some(username) && is_valid
                                            {
                                                // If authentication successful, select database
                                                authenticated = true;
                                                current_db_instance =
                                                    Some(Arc::new(db_instance.clone()));
                                                if let Err(e) = writer.write_all(format!("Authentication successful Using database '{}'\n", db_name).as_bytes()).await {
                                                    eprintln!("Error writing to socket: {}", e);
                                                    break;
                                                }
                                            } else {
                                                // If authentication failed, try again
                                                if let Err(e) = writer
                                                    .write_all(
                                                        b"Authentication failed. Try again.\n",
                                                    )
                                                    .await
                                                {
                                                    eprintln!("Error writing to socket: {}", e);
                                                    break;
                                                }
                                            }
                                        }
                                        // If authentication failed after max attempts, disconnect
                                        if !authenticated && auth_attempts >= MAX_AUTH_ATTEMPTS {
                                            if let Err(e) = writer.write_all(b"Too many failed authentication attempts. Disconnecting.\n").await {
                                                eprintln!("Error writing to socket: {}", e);
                                            }
                                            break;
                                        }
                                    } else {
                                        // If authentication is not required, select database
                                        current_db_instance = Some(Arc::new(db_instance.clone()));
                                        if let Err(e) = writer
                                            .write_all(
                                                format!("Using database '{}'\n", db_name)
                                                    .as_bytes(),
                                            )
                                            .await
                                        {
                                            eprintln!("Error writing to socket: {}", e);
                                            break;
                                        }
                                    }
                                }
                                None => {
                                    if let Err(e) = writer
                                        .write_all(
                                            format!("Database '{}' not found\n", db_name)
                                                .as_bytes(),
                                        )
                                        .await
                                    {
                                        eprintln!("Error writing to socket: {}", e);
                                        break;
                                    }
                                }
                            }
                        }
                    }
                    // Drop (delete) a database
                    "drop" if parts.len() == 2 => {
                        let db_name = parts[1].to_string();

                        // Check if trying to drop the currently selected database
                        if let Some(ref current_db) = current_db_instance {
                            if current_db.name == db_name {
                                if let Err(e) = writer.write_all(
                    b"Cannot drop the currently selected database. Please 'use' another database first.\n"
                ).await {
                    eprintln!("Error writing to socket: {}", e);
                    break;
                }
                                continue;
                            }
                        }

                        // First check if database exists without holding the lock across await
                        let (db_instance, exists_in_memory) = {
                            let mut dbs = all_dbs.lock().unwrap();
                            if let Some(db) = dbs.remove(&db_name) {
                                (Some(db), true)
                            } else {
                                (None, false)
                            }
                        };

                        // Handle file-based database case
                        let (db_instance, exists_in_memory) = if db_instance.is_none() {
                            if Path::new(&format!("dbs/{}.json", db_name)).exists() {
                                (db::DbInstance::load_from_file(&db_name), false)
                            } else {
                                (None, false)
                            }
                        } else {
                            (db_instance, exists_in_memory)
                        };

                        match db_instance {
                            Some(db_instance) => {
                                // Clone auth details before any awaits
                                let require_auth = db_instance.require_auth;
                                
                                // Handle authentication if required
                                if require_auth {
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
                                        let input_username = username_line.trim();

                                        if let Err(e) = writer.write_all(b"Password:\n").await {
                                            eprintln!("Error writing to socket: {}", e);
                                            break;
                                        }

                                        let mut password_line = String::new();
                                        if let Err(e) = reader.read_line(&mut password_line).await {
                                            eprintln!("Error reading password: {}", e);
                                            break;
                                        }
                                        let input_password = password_line.trim();
                                        let is_valid = match verify(input_password, db_instance.password.as_deref().unwrap_or("")) {
                                            Ok(valid) => valid,
                                            Err(e) => {
                                                eprintln!("Error verifying password: {}", e);
                                                if let Err(e) = writer.write_all(b"Authentication error.\n").await {
                                                    eprintln!("Error writing to socket: {}", e);
                                                }
                                                break;
                                            }
                                        };
                                        
                                        if db_instance.username.as_deref() == Some(input_username) && is_valid
                                        {
                                            authenticated = true;
                                        } else {
                                            if let Err(e) = writer
                                                .write_all(b"Authentication failed. Try again.\n")
                                                .await
                                            {
                                                eprintln!("Error writing to socket: {}", e);
                                                break;
                                            }
                                        }
                                    }

                                    if !authenticated {
                                        // Reinsert if it was in memory
                                        if exists_in_memory {
                                            let mut dbs = all_dbs.lock().unwrap();
                                            dbs.insert(db_name.clone(), db_instance);
                                        }
                                        if let Err(e) = writer.write_all(
                            b"Too many failed authentication attempts. Operation aborted.\n"
                        ).await {
                            eprintln!("Error writing to socket: {}", e);
                        }
                                        continue;
                                    }
                                }

                                // Delete the database file
                                let path = format!("dbs/{}.json", db_name);
                                if let Err(e) = std::fs::remove_file(&path) {
                                    // Reinsert if it was in memory
                                    if exists_in_memory {
                                        let mut dbs = all_dbs.lock().unwrap();
                                        dbs.insert(db_name.clone(), db_instance);
                                    }
                                    if let Err(e) = writer
                                        .write_all(
                                            format!("Error deleting database file: {}\n", e)
                                                .as_bytes(),
                                        )
                                        .await
                                    {
                                        eprintln!("Error writing to socket: {}", e);
                                        break;
                                    }
                                    continue;
                                }

                                if let Err(e) = writer
                                    .write_all(
                                        format!("Database '{}' deleted successfully\n", db_name)
                                            .as_bytes(),
                                    )
                                    .await
                                {
                                    eprintln!("Error writing to socket: {}", e);
                                    break;
                                }
                            }
                            None => {
                                if let Err(e) = writer
                                    .write_all(
                                        format!("Database '{}' not found\n", db_name).as_bytes(),
                                    )
                                    .await
                                {
                                    eprintln!("Error writing to socket: {}", e);
                                    break;
                                }
                            }
                        }
                    }
                    // All other commands
                    _ => {
                        match &current_db_instance {
                            Some(_db) => {
                                // Parse command and execute
                                let response =
                                    parser::parse_statement(line.trim(), &current_db_instance);
                                if let Err(e) =
                                    writer.write_all(format!("{}\n", response).as_bytes()).await
                                {
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
