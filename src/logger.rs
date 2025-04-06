use chrono::Local;
use std::fs::{OpenOptions};
use std::io::Write;

pub fn log_info(message: &str) {
    let now = Local::now();
    let formatted = format!("[INFO {}] {}", now.format("%Y-%m-%d %H:%M:%S"), message);

    // Print to console
    println!("{}", &formatted);

   
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open("output.log")
        .expect("Failed to open or create output.log");

    writeln!(file, "{}", &formatted).expect("Failed to write to output.log");
}
