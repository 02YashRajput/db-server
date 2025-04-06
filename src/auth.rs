
pub fn authenticate(username: &str, password: &str) -> bool {
  let valid_user = "admin";
  let valid_pass = "admin123";

  username == valid_user && password == valid_pass
}