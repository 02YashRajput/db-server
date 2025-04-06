use crate::db::DbInstance;

pub fn parse_statement(input: &str, _db: &DbInstance) -> String {
  input.trim().to_string()
}