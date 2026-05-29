//! Simplified single-pane output for Continuum.
//!
//! Provides clean, Claude Code-style output for the REPL.
//! Shows progress, results, and status in a readable format.

#![allow(dead_code)]

use std::io::{self, Write};

/// Safely truncate a string to `max` characters, adding ellipsis if truncated.
/// Uses char boundaries to avoid UTF-8 panics.
pub fn truncate_str(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let truncated: String = s.chars().take(max - 1).collect();
        format!("{truncated}…")
    }
}

/// Mask an API key, showing only first 6 chars.
pub fn mask_key(key: &str) -> String {
    let chars: Vec<char> = key.chars().collect();
    if chars.len() <= 8 {
        return "****".to_string();
    }
    let visible: String = chars[..6].iter().collect();
    format!("{visible}…{}", "*".repeat(6))
}

/// A simple progress indicator.
pub struct Progress {
    message: String,
    start_time: std::time::Instant,
}

impl Progress {
    /// Create a new progress indicator.
    pub fn new(message: &str) -> Self {
        print!("  ⟳ {message}...");
        let _ = io::stdout().flush();
        Self {
            message: message.to_string(),
            start_time: std::time::Instant::now(),
        }
    }

    /// Mark as complete with a success message.
    pub fn done(self, detail: &str) {
        let elapsed = self.start_time.elapsed();
        println!(
            "\r  ✓ {} — {} ({:.1}s)",
            self.message,
            detail,
            elapsed.as_secs_f64()
        );
    }

    /// Mark as complete with a success message (no timing).
    pub fn done_fast(self, detail: &str) {
        println!("\r  ✓ {} — {}", self.message, detail);
    }

    /// Mark as failed.
    pub fn failed(self, error: &str) {
        println!("\r  ✗ {} — {}", self.message, error);
    }
}

/// Print a section header.
pub fn section(title: &str) {
    println!();
    println!("  {title}");
    println!("  {}", "─".repeat(60));
}

/// Print a success message.
pub fn success(message: &str) {
    println!("  ✓ {message}");
}

/// Print an error message.
pub fn error(message: &str) {
    println!("  ✗ {message}");
}

/// Print an info message.
pub fn info(message: &str) {
    println!("  ℹ {message}");
}

/// Print a warning message.
pub fn warning(message: &str) {
    println!("  ⚠ {message}");
}

/// Print a key-value pair.
pub fn kv(key: &str, value: &str) {
    println!("  {:<16} {}", format!("{key}:"), value);
}

/// Print a result summary.
pub fn summary(items: &[(&str, &str)]) {
    println!();
    for (key, value) in items {
        kv(key, value);
    }
    println!();
}

/// Clear the current line and print a completion message.
pub fn complete_line(message: &str) {
    print!("\r{}\r", " ".repeat(80));
    println!("  {message}");
}

/// Print a box with a message.
pub fn box_message(title: &str, lines: &[&str]) {
    let width = lines
        .iter()
        .map(|l| l.len())
        .max()
        .unwrap_or(40)
        .max(title.len())
        + 4;
    let border = "═".repeat(width);

    println!();
    println!("  ┌{border}┐");
    println!("  │ {:<width$} │", title, width = width - 2);
    println!("  ├{border}┤");
    for line in lines {
        println!("  │ {:<width$} │", line, width = width - 2);
    }
    println!("  └{border}┘");
    println!();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_kv_format() {
        // Just verify it doesn't panic
        kv("test", "value");
    }

    #[test]
    fn test_success_format() {
        success("operation completed");
    }

    #[test]
    fn test_error_format() {
        error("something went wrong");
    }
}
