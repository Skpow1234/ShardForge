//! Interactive SQL REPL (Read-Eval-Print Loop)

use console::style;
use dialoguer::History;
use rustyline::error::ReadlineError;
use rustyline::{CompletionType, Config, Editor};
use rustyline::completion::{Completer, Pair};
use rustyline::highlight::Highlighter;
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::Helper;
use shardforge_core::Result;
use std::borrow::Cow;
use std::path::PathBuf;

use shardforge::sql::{executor::*, parser::SqlParser};
use shardforge_storage::{MemoryEngine, MVCCStorage};

use crate::commands::format_query_results;

/// SQL REPL history
struct SqlHistory {
    entries: Vec<String>,
}

impl SqlHistory {
    fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }
}

/// SQL keyword completer and helper
#[derive(Clone)]
struct SqlHelper {
    keywords: Vec<String>,
}

impl SqlHelper {
    fn new() -> Self {
        let keywords = vec![
            "SELECT", "FROM", "WHERE", "INSERT", "INTO", "VALUES", "UPDATE", "SET",
            "DELETE", "CREATE", "TABLE", "DROP", "ALTER", "INDEX", "PRIMARY", "KEY",
            "FOREIGN", "REFERENCES", "UNIQUE", "CHECK", "NOT", "NULL", "DEFAULT",
            "AUTO_INCREMENT", "BEGIN", "COMMIT", "ROLLBACK", "GROUP", "BY", "HAVING",
            "ORDER", "LIMIT", "OFFSET", "DISTINCT", "COUNT", "SUM", "AVG", "MIN", "MAX",
            "AND", "OR", "LIKE", "IN", "BETWEEN", "IS", "ASC", "DESC",
        ]
        .into_iter()
        .map(String::from)
        .collect();

        Self { keywords }
    }
}

impl Helper for SqlHelper {}

impl Completer for SqlHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &rustyline::Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let start = line[..pos]
            .rfind(char::is_whitespace)
            .map(|i| i + 1)
            .unwrap_or(0);

        let word = &line[start..pos];
        let word_upper = word.to_uppercase();

        let candidates: Vec<Pair> = self
            .keywords
            .iter()
            .filter(|k| k.starts_with(&word_upper))
            .map(|k| Pair {
                display: k.clone(),
                replacement: k.clone(),
            })
            .collect();

        Ok((start, candidates))
    }
}

impl Hinter for SqlHelper {
    type Hint = String;

    fn hint(&self, line: &str, pos: usize, _ctx: &rustyline::Context<'_>) -> Option<String> {
        if line.is_empty() || pos < line.len() {
            return None;
        }

        let word = line.split_whitespace().last().unwrap_or("");
        let word_upper = word.to_uppercase();

        self.keywords
            .iter()
            .find(|k| k.starts_with(&word_upper) && k.as_str() != word_upper.as_str())
            .map(|k| k[word.len()..].to_string())
    }
}

impl Highlighter for SqlHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Borrowed(line)
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _forced: bool) -> bool {
        false
    }
}

impl Validator for SqlHelper {}

/// Interactive SQL REPL
pub struct SqlRepl {
    editor: Editor<SqlHelper, rustyline::history::FileHistory>,
    parser: SqlParser,
    executor: QueryExecutor,
    database_name: String,
    history_path: PathBuf,
}

impl SqlRepl {
    /// Create a new SQL REPL
    pub async fn new(database_name: String) -> Result<Self> {
        let config = Config::builder()
            .history_ignore_space(true)
            .completion_type(CompletionType::List)
            .build();

        let helper = SqlHelper::new();
        let mut editor = Editor::with_config(config)?;
        editor.set_helper(Some(helper));

        // Setup history file
        let history_path = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("shardforge")
            .join("history.txt");

        if let Some(parent) = history_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        if history_path.exists() {
            editor.load_history(&history_path).ok();
        }

        // Create execution context
        let storage = Box::new(MemoryEngine::new(&Default::default()).await?);
        let mvcc = MVCCStorage::new();
        let catalog = SchemaCatalog::new();

        let context = ExecutionContext {
            storage,
            mvcc,
            catalog,
        };

        let executor = QueryExecutor::new(context);

        Ok(Self {
            editor,
            parser: SqlParser::new(),
            executor,
            database_name,
            history_path,
        })
    }

    /// Run the REPL
    pub async fn run(&mut self) -> Result<()> {
        self.print_welcome();

        let mut multi_line_buffer = String::new();

        loop {
            let prompt = if multi_line_buffer.is_empty() {
                format!("{}> ", style(&self.database_name).cyan().bold())
            } else {
                format!("{}→ ", style(&self.database_name).cyan().bold())
            };

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let trimmed = line.trim();

                    // Handle special commands
                    if trimmed.starts_with('\\') {
                        if let Err(e) = self.handle_meta_command(trimmed).await {
                            eprintln!("{} {}", style("Error:").red().bold(), e);
                        }
                        continue;
                    }

                    // Skip empty lines
                    if trimmed.is_empty() {
                        continue;
                    }

                    // Add to multi-line buffer
                    multi_line_buffer.push_str(trimmed);
                    multi_line_buffer.push(' ');

                    // Check if statement is complete (ends with ;)
                    if trimmed.ends_with(';') {
                        self.editor.add_history_entry(&multi_line_buffer)?;

                        // Execute the statement
                        if let Err(e) = self.execute_statement(&multi_line_buffer).await {
                            eprintln!("{} {}", style("Error:").red().bold(), e);
                        }

                        multi_line_buffer.clear();
                    }
                }
                Err(ReadlineError::Interrupted) => {
                    println!("^C");
                    multi_line_buffer.clear();
                }
                Err(ReadlineError::Eof) => {
                    println!("Bye!");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {:?}", err);
                    break;
                }
            }
        }

        // Save history
        self.editor.save_history(&self.history_path).ok();

        Ok(())
    }

    /// Print welcome message
    fn print_welcome(&self) {
        println!("{}", style("ShardForge Interactive SQL Shell").green().bold());
        println!("{}", style("=====================================").green());
        println!("Type SQL statements to execute them.");
        println!("Type {} for help.", style("\\help").cyan());
        println!("Type {} or press Ctrl+D to exit.", style("\\quit").cyan());
        println!();
    }

    /// Handle meta commands (commands starting with \)
    async fn handle_meta_command(&mut self, command: &str) -> Result<()> {
        match command.to_lowercase().as_str() {
            "\\quit" | "\\q" | "\\exit" => {
                println!("Bye!");
                std::process::exit(0);
            }
            "\\help" | "\\?" => {
                self.print_help();
            }
            "\\tables" | "\\dt" => {
                self.list_tables();
            }
            "\\clear" | "\\c" => {
                // Clear screen
                print!("\x1B[2J\x1B[1;1H");
            }
            _ => {
                println!("Unknown command: {}. Type \\help for help.", command);
            }
        }

        Ok(())
    }

    /// Execute a SQL statement
    async fn execute_statement(&mut self, sql: &str) -> Result<()> {
        let start = std::time::Instant::now();

        // Parse the statement
        let statement = self.parser.parse(sql)?;

        // Execute the statement
        let result = self.executor.execute(statement).await?;

        let elapsed = start.elapsed();

        // Display results
        if !result.columns.is_empty() {
            println!("{}", format_query_results(&result.columns, &result.rows));
        }

        // Display summary
        println!(
            "{} {} row(s) in {:.3}s",
            style("→").green(),
            style(result.affected_rows).yellow().bold(),
            elapsed.as_secs_f64()
        );

        Ok(())
    }

    /// Print help message
    fn print_help(&self) {
        println!("{}", style("Meta Commands:").green().bold());
        println!("  \\help, \\?     Show this help message");
        println!("  \\quit, \\q     Exit the shell");
        println!("  \\tables, \\dt  List all tables");
        println!("  \\clear, \\c    Clear the screen");
        println!();
        println!("{}", style("SQL Commands:").green().bold());
        println!("  CREATE TABLE   Create a new table");
        println!("  DROP TABLE     Delete a table");
        println!("  INSERT INTO    Insert data into a table");
        println!("  SELECT         Query data from tables");
        println!("  UPDATE         Update existing data");
        println!("  DELETE         Delete data from a table");
        println!();
        println!("{}", style("Examples:").green().bold());
        println!("  CREATE TABLE users (id INTEGER, name VARCHAR(255));");
        println!("  INSERT INTO users VALUES (1, 'Alice');");
        println!("  SELECT * FROM users;");
        println!();
    }

    /// List all tables
    fn list_tables(&self) {
        let tables = self.executor.context.catalog.list_tables();

        if tables.is_empty() {
            println!("{}", style("No tables found.").yellow());
        } else {
            println!("{}", style("Tables:").green().bold());
            for table in tables {
                println!("  • {}", table);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_helper_completion() {
        let helper = SqlHelper::new();
        let (start, candidates) = helper.complete("SEL", 3, &rustyline::Context::new(&rustyline::history::FileHistory::new())).unwrap();
        
        assert_eq!(start, 0);
        assert!(candidates.iter().any(|c| c.display == "SELECT"));
    }
}

