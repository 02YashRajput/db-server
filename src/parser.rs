use crate::db::{DbInstance, ValueWithExpiry};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

pub fn parse_statement(input: &str, current_db: &Option<Arc<Mutex<HashMap<String, ValueWithExpiry>>>>) -> String {
    match current_db {
        Some(db_instance) => {
            let db = db_instance.lock().unwrap();
            match input.trim() {
                "show" => {
                    // Show the number of entries in the database
                    return format!("Database contains {} entries", db.len());
                }
                "list" => {
                    // List all keys in the database
                    let keys: Vec<String> = db.keys().cloned().collect();
                    if keys.is_empty() {
                        return "No keys in the database".to_string();
                    }
                    return format!("Keys in the database: {:?}", keys);
                }
                cmd if cmd.starts_with("get") => {
                    // Get a value by key (e.g., get <key>)
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    if parts.len() == 2 {
                        let key = parts[1];
                        match db.get(key) {
                            Some(value_with_expiry) => {
                                if value_with_expiry.is_expired() {
                                    return format!("Key '{}' has expired", key);
                                }
                                return format!("Value for '{}': {}", key, value_with_expiry.value);
                            }
                            None => format!("Key '{}' not found", key),
                        }
                    } else {
                        "Invalid command. Usage: get <key>".to_string()
                    }
                }
                cmd if cmd.starts_with("set") => {
                    // Set a value with optional TTL (e.g., set <key> <value> [ttl_seconds])
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let key = parts[1];
                        let value = parts[2].to_string();
                        let ttl = if parts.len() == 4 {
                            Some(parts[3].parse::<u64>().unwrap_or(0)) // TTL in seconds
                        } else {
                            None
                        };
                        let ttl_duration = ttl.map(|t| std::time::Duration::from_secs(t));
                        let new_value = ValueWithExpiry::new(value, ttl_duration);
                        
                        let mut db = db_instance.lock().unwrap();
                        db.insert(key.to_string(), new_value);
                        
                        return format!("Set value for '{}'", key);
                    } else {
                        return "Invalid command. Usage: set <key> <value> [ttl_seconds]".to_string();
                    }
                }
                cmd if cmd.starts_with("delete") => {
                    // Delete a key (e.g., delete <key>)
                    let parts: Vec<&str> = cmd.split_whitespace().collect();
                    if parts.len() == 2 {
                        let key = parts[1];
                        let mut db = db_instance.lock().unwrap();
                        if db.remove(key).is_some() {
                            return format!("Key '{}' deleted", key);
                        } else {
                            return format!("Key '{}' not found", key);
                        }
                    } else {
                        return "Invalid command. Usage: delete <key>".to_string();
                    }
                }
                _ => "Unknown command".to_string(),
            }
        }
        None => "No database selected".to_string(),
    }
}
