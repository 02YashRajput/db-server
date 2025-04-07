use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::Path;
use serde::{Serialize, Deserialize};

use crate::logger::log_info;

// Type alias for a database: a thread-safe, shared, mutable map of key-value pairs.
pub type Db = Arc<Mutex<HashMap<String, ValueWithExpiry>>>;

// Type alias for managing multiple databases: each identified by a name and associated with a `DbInstance`.
pub type DbMap = Arc<Mutex<HashMap<String, DbInstance>>>;

/// Represents a single database instance.
#[derive(Debug, Clone)]
pub struct DbInstance {
    // The actual data in the DB, stored with expiration support.
    pub data: Db,
    // Whether authentication is required to use this database.
    pub require_auth: bool,
    // Optional username for authentication.
    pub username: Option<String>,
    // Optional password for authentication.
    pub password: Option<String>,
    // Database name
    pub name: String,
}

// Serializable version of ValueWithExpiry for JSON storage
#[derive(Serialize, Deserialize, Debug)]
struct SerializableValueWithExpiry {
    value: String,
    expires_at: Option<u64>, // Stored as timestamp in seconds
}

// Serializable version of database for JSON storage
#[derive(Serialize, Deserialize, Debug)]
struct SerializableDb {
    data: HashMap<String, SerializableValueWithExpiry>,
    require_auth: bool,
    username: Option<String>,
    password: Option<String>,
}

impl DbInstance {
    /// Creates a new database instance and persists it to a file.
    pub fn new(name: String, require_auth: bool, username: Option<String>, password: Option<String>) -> Self {
        // Create the dbs directory if it doesn't exist
        fs::create_dir_all("dbs").unwrap_or_else(|_| ());
        
        let instance = Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            require_auth,
            username,
            password,
            name,
        };
        
        // Save empty database to file
        instance.save_to_file().expect("Failed to save new database");
        instance
    }

    /// Loads a database from file
    pub fn load_from_file(name: &str) -> Option<Self> {
        let path = format!("dbs/{}.json", name);
        if !Path::new(&path).exists() {
            return None;
        }

        let mut file = File::open(&path).ok()?;
        let mut contents = String::new();
        file.read_to_string(&mut contents).ok()?;
        
        let serialized: SerializableDb = serde_json::from_str(&contents).ok()?;
        
        let mut data = HashMap::new();
        for (key, val) in serialized.data {
            let expires_at = val.expires_at.map(|ts| {
                Instant::now() + Duration::from_secs(ts.saturating_sub(
                    Instant::now().elapsed().as_secs()
                ))
            });
            
            data.insert(key, ValueWithExpiry {
                value: val.value,
                expires_at,
            });
        }

        Some(Self {
            data: Arc::new(Mutex::new(data)),
            require_auth: serialized.require_auth,
            username: serialized.username,
            password: serialized.password,
            name: name.to_string(),
        })
    }

    /// Saves the database to file
    pub fn save_to_file(&self) -> std::io::Result<()> {
        let path = format!("dbs/{}.json", self.name);
        
        let data = self.data.lock().unwrap();
        
        let mut serialized_data = HashMap::new();
        for (key, val) in data.iter() {
            let expires_at = val.expires_at.map(|instant| {
                instant.checked_duration_since(Instant::now())
                    .map(|dur| dur.as_secs())
                    .unwrap_or(0)
            });
            
            serialized_data.insert(
                key.clone(),
                SerializableValueWithExpiry {
                    value: val.value.clone(),
                    expires_at,
                }
            );
        }
        
        
        let serialized = SerializableDb {
            data: serialized_data,
            require_auth: self.require_auth,
            username: self.username.clone(),
            password: self.password.clone(),
        };
        
        
        let json = match serde_json::to_string_pretty(&serialized) {
            Ok(j) => j,
            Err(e) => {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, e));
            }
        };
        
        
        match File::create(&path) {
            Ok(mut file) => {
                if let Err(e) = file.write_all(json.as_bytes()) {
                    return Err(e);
                }
                Ok(())
            }
            Err(e) => {
                Err(e)
            }
        }
    }

    pub fn persist(&self) {
        if let Err(e) = self.save_to_file() {
            log_info(&format!("⚠️ Failed to persist database '{}': {}", self.name, e));
        }
    }
}

/// Represents a value in the database along with its optional expiration time.
#[derive(Debug, Clone)]
pub struct ValueWithExpiry {
    // The actual value stored in the DB.
    pub value: String,
    // When the key should expire (if any).
    pub expires_at: Option<Instant>, 
}

impl ValueWithExpiry {
    /// Creates a new `ValueWithExpiry` with optional time-to-live.
    pub fn new(value: String, ttl: Option<Duration>) -> Self {
        // Calculate the expiry time if TTL is provided.
        let expires_at = ttl.map(|d| Instant::now() + d);

        // Log key insertion with TTL status.
        let msg = match expires_at {
            Some(time) => format!("New key inserted with TTL ({:?})", time),
            None => "New key inserted with no TTL".to_string(),
        };
        log_info(&msg);

        Self { value, expires_at }
    }

    /// Checks if the value has expired based on current time.
    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|time| Instant::now() > time)
            .unwrap_or(false)
    }
}