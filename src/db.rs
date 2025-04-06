// db.rs
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::logger::log_info;

pub type Db = Arc<Mutex<HashMap<String, ValueWithExpiry>>>;

#[derive(Debug, Clone)]
pub struct ValueWithExpiry {
    pub value: String,
    pub expires_at: Option<Instant>, // None = no expiry
}

impl ValueWithExpiry {
    pub fn new(value: String, ttl: Option<Duration>) -> Self {
        let expires_at = ttl.map(|d| Instant::now() + d);
        let msg = match expires_at {
            Some(time) => format!("New key inserted with TTL ({:?})", time),
            None => "New key inserted with no TTL".to_string(),
        };
        log_info(&msg);
        Self { value, expires_at }
    }

    pub fn is_expired(&self) -> bool {
        self.expires_at
            .map(|time| Instant::now() > time)
            .unwrap_or(false)
    }
}
