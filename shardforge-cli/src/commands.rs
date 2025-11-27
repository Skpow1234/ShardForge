//! CLI command implementations

use comfy_table::{modifiers::UTF8_ROUND_CORNERS, presets::UTF8_FULL, Cell, Color, Table};
use console::style;
use dialoguer::{Confirm, Input};
use indicatif::{ProgressBar, ProgressStyle};
use shardforge_core::{Result, ShardForgeError};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

pub use crate::import_export::*;

/// Format query results as a table
pub fn format_query_results(columns: &[String], rows: &[Vec<shardforge_core::Value>]) -> String {
    let mut table = Table::new();
    table
        .load_preset(UTF8_FULL)
        .apply_modifier(UTF8_ROUND_CORNERS)
        .set_header(columns.iter().map(|s| {
            Cell::new(s).fg(Color::Green)
        }));

    for row in rows {
        let cells: Vec<Cell> = row
            .iter()
            .map(|value| {
                let bytes = value.as_ref();
                if bytes.is_empty() {
                    Cell::new("NULL").fg(Color::DarkGrey)
                } else if let Ok(s) = std::str::from_utf8(bytes) {
                    Cell::new(s)
                } else {
                    Cell::new(format!("<binary:{}>", bytes.len()))
                }
            })
            .collect();
        table.add_row(cells);
    }

    table.to_string()
}

/// Execute SQL from a file
pub fn execute_sql_file<P: AsRef<Path>>(file_path: P) -> Result<Vec<String>> {
    let file = File::open(file_path).map_err(|e| ShardForgeError::Io {
        message: format!("Failed to open SQL file: {}", e),
    })?;

    let reader = BufReader::new(file);
    let mut statements = Vec::new();
    let mut current_statement = String::new();

    for line_result in reader.lines() {
        let line = line_result.map_err(|e| ShardForgeError::Io {
            message: format!("Failed to read line: {}", e),
        })?;

        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("--") || trimmed.is_empty() {
            continue;
        }

        current_statement.push_str(&line);
        current_statement.push(' ');

        // Check for statement terminator
        if trimmed.ends_with(';') {
            statements.push(current_statement.trim().to_string());
            current_statement.clear();
        }
    }

    // Add final statement if not terminated
    if !current_statement.is_empty() {
        statements.push(current_statement.trim().to_string());
    }

    Ok(statements)
}

/// Show progress bar for operations
pub fn create_progress_bar(message: &str, length: u64) -> ProgressBar {
    let pb = ProgressBar::new(length);
    pb.set_style(
        ProgressStyle::default_bar()
            .template("{msg}\n{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {pos}/{len} ({eta})")
            .unwrap()
            .progress_chars("#>-"),
    );
    pb.set_message(message.to_string());
    pb
}

/// Confirm action with user
pub fn confirm_action(message: &str) -> Result<bool> {
    Confirm::new()
        .with_prompt(message)
        .interact()
        .map_err(|e| ShardForgeError::Io {
            message: format!("Failed to get confirmation: {}", e),
        })
}

/// Get input from user
pub fn get_input(prompt: &str) -> Result<String> {
    Input::<String>::new()
        .with_prompt(prompt)
        .interact_text()
        .map_err(|e| ShardForgeError::Io {
            message: format!("Failed to get input: {}", e),
        })
}

/// Print success message
pub fn print_success(message: &str) {
    println!("{} {}", style("✓").green().bold(), style(message).green());
}

/// Print error message
pub fn print_error(message: &str) {
    eprintln!("{} {}", style("✗").red().bold(), style(message).red());
}

/// Print info message
pub fn print_info(message: &str) {
    println!("{} {}", style("ℹ").blue().bold(), message);
}

/// Print warning message
pub fn print_warning(message: &str) {
    println!("{} {}", style("⚠").yellow().bold(), style(message).yellow());
}
