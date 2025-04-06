use std::time::Duration;
use tokio::time::sleep;

use crate::db::{DbMap};
use crate::logger::log_info;

pub async fn start_cleaner(db_map: DbMap) {
    tokio::spawn(async move {
        loop {
            {
                let db_map_lock = db_map.lock().unwrap();

                for (db_name, db_instance) in db_map_lock.iter() {
                    let mut data_lock = db_instance.data.lock().unwrap();

                    // Collect expired keys
                    let expired_keys: Vec<String> = data_lock
                        .iter()
                        .filter(|(_, v)| v.is_expired())
                        .map(|(k, _)| k.clone())
                        .collect();

                    // Remove expired keys
                    for key in &expired_keys {
                        data_lock.remove(key);
                    }

                    if !expired_keys.is_empty() {
                        log_info(&format!(
                            "🧼 Cleaned {} expired keys from '{}': [{}]",
                            expired_keys.len(),
                            db_name,
                            expired_keys.join(", ")
                        ));
                    }
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    });
}
