//! ShardForge Command Line Interface

use clap::{Parser, Subcommand};
use shardforge::core::Result;
use std::process;

#[derive(Parser)]
#[command(name = "shardforge")]
#[command(about = "ShardForge Distributed Database")]
#[command(version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize a new ShardForge instance
    Init {
        /// Data directory path
        #[arg(short, long, default_value = "./data")]
        data_dir: String,

        /// Single-node mode (no clustering)
        #[arg(long)]
        single_node: bool,
    },

    /// Start the ShardForge database server
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "shardforge.toml")]
        config: String,
    },

    /// Show database status
    Status {
        /// Show detailed status
        #[arg(long)]
        detailed: bool,
    },

    /// Execute SQL queries
    Sql {
        /// SQL query to execute
        query: Option<String>,

        /// Database name
        #[arg(short, long, default_value = "default")]
        database: String,
    },

    /// Show version information
    Version,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let result = match cli.command {
        Some(Commands::Init { data_dir, single_node }) => {
            handle_init(&data_dir, single_node).await
        }
        Some(Commands::Start { config }) => {
            handle_start(&config).await
        }
        Some(Commands::Status { detailed }) => {
            handle_status(detailed).await
        }
        Some(Commands::Sql { query, database }) => {
            handle_sql(query.as_deref(), &database).await
        }
        Some(Commands::Version) => {
            handle_version().await
        }
        None => {
            // No subcommand provided, show help
            let _ = Cli::command().print_help();
            Ok(())
        }
    };

    if let Err(e) = result {
        eprintln!("Error: {}", e);
        process::exit(1);
    }
}

async fn handle_init(data_dir: &str, single_node: bool) -> Result<()> {
    println!("Initializing ShardForge...");
    println!("Data directory: {}", data_dir);
    println!("Single node: {}", single_node);

    // Create data directory if it doesn't exist
    std::fs::create_dir_all(data_dir).map_err(|e| {
        shardforge::core::ShardForgeError::Internal(
            format!("Failed to create data directory: {}", e)
        )
    })?;

    // Create default configuration file
    let config_path = std::path::Path::new(data_dir).join("../shardforge.toml");
    if !config_path.exists() {
        let default_config = r#"
[cluster]
name = "default-cluster"
data_directory = "${DATA_DIR}"

[node]
bind_address = "127.0.0.1:5432"
max_connections = 100

[storage]
engine = "memory"

[logging]
level = "info"
format = "pretty"
"#.replace("${DATA_DIR}", data_dir);

        std::fs::write(&config_path, default_config).map_err(|e| {
            shardforge::core::ShardForgeError::Internal(
                format!("Failed to create config file: {}", e)
            )
        })?;
        println!("Created default configuration at: {}", config_path.display());
    }

    println!("ShardForge initialized successfully!");
    println!("You can now start the database with: shardforge start");
    Ok(())
}

async fn handle_start(config_path: &str) -> Result<()> {
    println!("Starting ShardForge...");
    println!("Configuration: {}", config_path);

    // For now, just show that the command would start the server
    // In the future, this will actually start the database server
    println!("Database server would start here with config: {}", config_path);
    println!("Note: Full server implementation coming in Phase 2");
    Ok(())
}

async fn handle_status(detailed: bool) -> Result<()> {
    println!("ShardForge Status");
    println!("================");

    if detailed {
        println!("Version: {}", shardforge::VERSION);
        println!("Build: {}", shardforge::BUILD_INFO);
        println!("Status: Not running (server implementation pending)");
        println!("Storage: Memory engine (default)");
        println!("Network: Not listening (server implementation pending)");
    } else {
        println!("Status: Ready for development");
        println!("Version: {}", shardforge::VERSION);
    }

    Ok(())
}

async fn handle_sql(query: Option<&str>, database: &str) -> Result<()> {
    println!("SQL Execution");
    println!("=============");
    println!("Database: {}", database);

    if let Some(q) = query {
        println!("Query: {}", q);
        println!("Note: SQL execution engine coming in Phase 2");
    } else {
        println!("Interactive mode: Not yet implemented");
        println!("Use: shardforge sql --query \"SELECT 1\"");
    }

    Ok(())
}

async fn handle_version() -> Result<()> {
    println!("ShardForge {}", shardforge::VERSION);
    println!("Build: {}", shardforge::BUILD_INFO);
    println!("Phase: 1 (Foundation) - Core Infrastructure Complete");
    Ok(())
}
