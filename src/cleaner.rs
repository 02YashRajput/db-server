use std::time::Duration;
use tokio::time::sleep;

use crate::db::Db;
use crate::logger::log_info;

pub async fn start_cleaner(db: Db) {
    tokio::spawn(async move {
        loop {
            {
                let mut db_lock = db.lock().unwrap();

                // Collect expired keys
                let expired_keys: Vec<String> = db_lock
                    .iter()
                    .filter(|(_, v)| v.is_expired())
                    .map(|(k, _)| k.clone())

                    .collect();

                // Remove expired keys
                for key in &expired_keys {
                    db_lock.remove(key);
                }

                if !expired_keys.is_empty() {
                    log_info(&format!(
                        "🧼 Cleaned {} expired keys: [{}]",
                        expired_keys.len(),
                        expired_keys.join(", ")
                    ));
                }
            }

            sleep(Duration::from_secs(5)).await;
        }
    });
}
