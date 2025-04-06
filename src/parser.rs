use crate::db::{Db, ValueWithExpiry};
use std::time::Duration;


fn parse_duration(s: &str) -> Result<std::time::Duration, String> {
  if s.is_empty() {
      return Err("Empty TTL provided".to_string());
  }

  let num_part: String = s.chars().take_while(|c| c.is_digit(10)).collect();
  let unit_part: String = s.chars().skip_while(|c| c.is_digit(10)).collect();

  let num = num_part.parse::<u64>().map_err(|_| "Invalid TTL number".to_string())?;

  match unit_part.as_str() {
      "s" => Ok(std::time::Duration::from_secs(num)),
      "m" => Ok(std::time::Duration::from_secs(num * 60)),
      "d" => Ok(std::time::Duration::from_secs(num * 60 * 60 * 24)),
      _ => Err("Invalid TTL unit (use s, m, or d)".to_string()),
  }
}


pub fn parse_statement(input: &str, db: &Db) -> String {
  let input = input.trim();

  if input.starts_with("SET(") && input.ends_with(')') {
      let content = &input[4..input.len() - 1];
      let args: Vec<&str> = content
          .split(',')
          .map(|s| s.trim().trim_matches('"'))
          .collect();

      if args.len() < 2 {
          return "Usage: SET(\"key\",\"value\",[\"5s|5m|5d\"])".to_string();
      }

      let key = args[0].to_string();
      let value = args[1].to_string();
      let mut ttl: Option<Duration> = None;

      if args.len() == 3 {
          ttl = match parse_duration(args[2]) {
              Ok(dur) => Some(dur),
              Err(e) => return e,
          };
      }

      let entry = ValueWithExpiry::new(value, ttl);
      let mut db = db.lock().unwrap();
      db.insert(key, entry);
      "OK".to_string()
  } else if input.starts_with("GET(") && input.ends_with(')') {
      let content = &input[4..input.len() - 1];
      let key = content.trim().trim_matches('"');

      let mut db = db.lock().unwrap();
      match db.get(key) {
          Some(val) if !val.is_expired() => val.value.clone(),
          Some(_) => {
              db.remove(key);
              format!("Error: Key \"{}\" not found", key)
          }
          None => format!("Error: Key \"{}\" not found", key),
      }
  } else if input.starts_with("DEL(") && input.ends_with(')') {
      let content = &input[4..input.len() - 1];
      let key = content.trim().trim_matches('"');

      let mut db = db.lock().unwrap();
      if db.remove(key).is_some() {
          "OK".to_string()
      } else {
          format!("Error: Key \"{}\" not found", key)
      }
  } else {
      "Unknown command".to_string()
  }
}
