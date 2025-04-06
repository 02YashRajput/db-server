use chrono::Local;
use std::fs::OpenOptions;
use std::io::Write;

/// Logs an info-level message to a file named `output.log`
/// Each log entry is timestamped with the local date and time.
pub fn log_info(message: &str) {
    // Get the current local timestamp
    let now = Local::now();

    // Format the log message with timestamp and message content
    let formatted = format!("[INFO {}] {}", now.format("%Y-%m-%d %H:%M:%S"), message);

    // Open or create the log file in append mode
    let mut file = OpenOptions::new()
        .create(true)   // Create the file if it doesn't exist
        .append(true)   // Append to the file instead of overwriting it
        .open("output.log")
        .expect("Failed to open or create output.log");

    // Write the formatted message to the file, followed by a newline
    writeln!(file, "{}", &formatted).expect("Failed to write to output.log");
}
