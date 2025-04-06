use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

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
}

impl DbInstance {
    /// Creates a new database instance.
    /// If authentication is required, `username` and `password` must be provided.
    pub fn new(require_auth: bool, username: Option<String>, password: Option<String>) -> Self {
        Self {
            data: Arc::new(Mutex::new(HashMap::new())),
            require_auth,
            username,
            password,
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
