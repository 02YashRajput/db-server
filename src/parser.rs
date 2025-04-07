// =======================================================
// 🧠 INFO: Imports
// =======================================================
use crate::db::{DbInstance, ValueWithExpiry};
use std::sync::Arc;
use std::time::Duration;

/// Parses duration string (e.g. "5s", "10m", "1d") into Duration
/// Format: <number><unit> where unit is s (seconds), m (minutes), or d (days)
/// Returns error string if format is invalid
fn parse_duration(s: &str) -> Result<Duration, String> {
    if s.is_empty() {
        return Err("Empty TTL provided".to_string());
    }

    // Split into numeric and unit parts
    let num_part: String = s.chars().take_while(|c| c.is_digit(10)).collect();
    let unit_part: String = s.chars().skip_while(|c| c.is_digit(10)).collect();

    let num = num_part.parse::<u64>().map_err(|_| "Invalid TTL number".to_string())?;

    match unit_part.as_str() {
        "s" => Ok(Duration::from_secs(num)),
        "m" => Ok(Duration::from_secs(num * 60)),  // Convert minutes to seconds
        "d" => Ok(Duration::from_secs(num * 60 * 60 * 24)),  // Convert days to seconds
        _ => Err("Invalid TTL unit (use s, m, or d)".to_string()),
    }
}

// =======================================================
// 🧠 INFO: Main Command Parser
// =======================================================
/// Parses and executes database commands
/// Supported commands:
/// - SET("key","value",["ttl"]) - Stores key-value pair with optional TTL
/// - GET("key") - Retrieves value for key
/// - DEL("key") - Deletes key
pub fn parse_statement(input: &str,  current_db_instance: &Option<Arc<DbInstance>> ) -> String {
    let input = input.trim();

    // Handle SET command
    if input.starts_with("SET(") && input.ends_with(')') {
        let content = &input[4..input.len() - 1];  // Extract content between parentheses
        let args: Vec<&str> = content
            .split(',')
            .map(|s| s.trim().trim_matches('"'))  // Clean each argument
            .collect();

        if args.len() < 2 {
            return "Usage: SET(\"key\",\"value\",[\"5s|5m|5d\"])".to_string();
        }

        let key = args[0].to_string();
        let value = args[1].to_string();
        let mut ttl: Option<Duration> = None;

        // Parse TTL if provided
        if args.len() == 3 {
            ttl = match parse_duration(args[2]) {
                Ok(dur) => Some(dur),
                Err(e) => return e,  // Return error message if TTL parsing fails
            };
        }

        let entry = ValueWithExpiry::new(value, ttl);

        match current_db_instance {
            Some(db_instance) => {
                {
                    let mut db = db_instance.data.lock().unwrap();
                    db.insert(key, entry);
                }
                
                // Persist after releasing the lock
                db_instance.persist();
                "OK".to_string()
            }
            None => "No database selected".to_string(),
        }
    } 
    // Handle GET command
    else if input.starts_with("GET(") && input.ends_with(')') {
        let content = &input[4..input.len() - 1];
        let key = content.trim().trim_matches('"');

        match current_db_instance {
            Some(db_instance) => {
                let mut db = db_instance.data.lock().unwrap();
                match db.get(key) {
                    Some(val) if !val.is_expired() => val.value.clone(),
                    Some(_) => {
                        db.remove(key);
                        drop(db); 
                        db_instance.persist();
                        format!("Error: Key \"{}\" has expired and is deleted", key)
                    }
                    None => format!("Error: Key \"{}\" not found", key),
                }
            }
            None => "No database selected".to_string(),
        }
    } 
    // Handle DEL command
    else if input.starts_with("DEL(") && input.ends_with(')') {
        let content = &input[4..input.len() - 1];
        let key = content.trim().trim_matches('"');

        match current_db_instance {
            Some(db_instance) => {
                let removed = {
                    let mut db = db_instance.data.lock().unwrap();
                    db.remove(key).is_some()
                };
                
                // Persist only if key was actually removed
                if removed {
                    db_instance.persist();
                    "OK".to_string()
                } else {
                    format!("Error: Key \"{}\" not found", key)
                }
            }
            None => "No database selected".to_string(),
        }
    } else {
        "Unknown command".to_string()  // Fallback for invalid commands
    }
}