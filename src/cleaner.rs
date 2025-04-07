use std::time::Duration;
use tokio::time::sleep;

use crate::db::DbMap;
use crate::logger::log_info;

/// Starts a background async task that periodically scans all databases
/// in `db_map` and removes expired keys every 5 seconds.
pub async fn start_cleaner(db_map: DbMap) {
    // Spawn a new asynchronous task to run in the background
    tokio::spawn(async move {
        loop {
            {
                // Acquire a lock on the global database map
                let db_map_lock = db_map.lock().unwrap();

                // Iterate over each database instance
                for (db_name, db_instance) in db_map_lock.iter() {
                    // Lock the actual key-value data inside the database
                    let mut data_lock = db_instance.data.lock().unwrap();

                    // Collect all keys that have expired
                    let expired_keys: Vec<String> = data_lock
                        .iter()
                        .filter(|(_, v)| v.is_expired()) // Check if value is expired
                        .map(|(k, _)| k.clone()) // Collect the key
                        .collect();

                    // Remove all expired keys from the database
                    for key in &expired_keys {
                        data_lock.remove(key);
                    }

                    // Log the cleanup action if any keys were removed
                    if !expired_keys.is_empty() {
                        log_info(&format!(
                            "🧼 Cleaned {} expired keys from '{}': [{}]",
                            expired_keys.len(),
                            db_name,
                            expired_keys.join(", ")
                        ));
                    }
                    drop(data_lock);
                    db_instance.persist();
                }
            }

            // Sleep for 5 seconds before the next cleanup cycle
            sleep(Duration::from_secs(5)).await;
        }
    });
}
