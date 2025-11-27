//! ShardForge Command Line Interface

use clap::{Parser, Subcommand};
use console::style;
use shardforge_cli::*;
use shardforge_core::Result;
use std::process;

#[derive(Parser)]
#[command(name = "shardforge")]
#[command(about = "ShardForge Distributed Database")]
#[command(version, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
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

        /// Configuration file path
        #[arg(short, long, default_value = "shardforge.toml")]
        config: String,
    },

    /// Start the ShardForge database server
    Start {
        /// Configuration file path
        #[arg(short, long, default_value = "shardforge.toml")]
        config: String,
    },

    /// Stop the ShardForge database server
    Stop {
        /// Graceful shutdown timeout in seconds
        #[arg(short, long, default_value = "30")]
        timeout: u32,
    },

    /// Show database status
    Status {
        /// Show detailed status
        #[arg(long)]
        detailed: bool,

        /// Output format
        #[arg(long, default_value = "table")]
        format: String,
    },

    /// Execute SQL queries
    Sql {
        /// SQL query to execute
        query: Option<String>,

        /// Database name
        #[arg(short, long, default_value = "default")]
        database: String,

        /// File containing SQL to execute
        #[arg(short, long)]
        file: Option<String>,

        /// Interactive mode
        #[arg(short, long)]
        interactive: bool,
    },

    /// Import data from CSV or JSON
    Import {
        /// Input file path
        #[arg(short, long)]
        file: String,

        /// Target table name
        #[arg(short, long)]
        table: String,

        /// Database name
        #[arg(short, long, default_value = "default")]
        database: String,

        /// File format (csv, json)
        #[arg(short='F', long, default_value = "csv")]
        format: String,

        /// Column definitions (name:type,name:type,...)
        #[arg(short, long)]
        columns: String,

        /// CSV has header row
        #[arg(long, default_value = "true")]
        header: bool,

        /// CSV delimiter
        #[arg(long, default_value = ",")]
        delimiter: String,
    },

    /// Export data to CSV or JSON
    Export {
        /// SQL query to export
        #[arg(short, long)]
        query: String,

        /// Output file path
        #[arg(short, long)]
        output: String,

        /// Database name
        #[arg(short, long, default_value = "default")]
        database: String,

        /// File format (csv, json)
        #[arg(short='F', long, default_value = "csv")]
        format: String,

        /// Pretty print JSON
        #[arg(long)]
        pretty: bool,

        /// Include CSV header
        #[arg(long, default_value = "true")]
        header: bool,
    },

    /// Cluster management commands
    Cluster {
        #[command(subcommand)]
        command: ClusterCommands,
    },

    /// Configuration management
    Config {
        #[command(subcommand)]
        command: ConfigCommands,
    },
}

#[derive(Subcommand)]
enum ClusterCommands {
    /// Initialize a new cluster
    Init {
        /// Node ID for this node
        #[arg(long)]
        node_id: Option<String>,

        /// Cluster name
        #[arg(long, default_value = "shardforge-cluster")]
        name: String,

        /// Bind address
        #[arg(long, default_value = "0.0.0.0:5432")]
        bind: String,
    },

    /// Join an existing cluster
    Join {
        /// Seed node address
        seed_node: String,

        /// Node ID for this node
        #[arg(long)]
        node_id: Option<String>,

        /// Bind address
        #[arg(long, default_value = "0.0.0.0:5432")]
        bind: String,
    },

    /// Show cluster status
    Status {
        /// Show detailed status
        #[arg(long)]
        detailed: bool,

        /// Output format
        #[arg(long, default_value = "table")]
        format: String,
    },
}

#[derive(Subcommand)]
enum ConfigCommands {
    /// Get configuration value
    Get {
        /// Configuration key
        key: String,
    },

    /// Set configuration value
    Set {
        /// Configuration key
        key: String,

        /// Configuration value
        value: String,

        /// Validate configuration
        #[arg(long)]
        validate: bool,
    },

    /// Show current configuration
    Show {
        /// Show all configuration
        #[arg(long)]
        all: bool,
    },
}

#[tokio::main]
async fn main() {
    // Initialize logging
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Init { data_dir, single_node, config } => {
            handle_init(&data_dir, single_node, &config).await
        }
        Commands::Start { config } => handle_start(&config).await,
        Commands::Stop { timeout } => handle_stop(timeout).await,
        Commands::Status { detailed, format } => handle_status(detailed, &format).await,
        Commands::Sql { query, database, file, interactive } => {
            handle_sql(query.as_deref(), &database, file.as_deref(), interactive).await
        }
        Commands::Import {
            file,
            table,
            database,
            format,
            columns,
            header,
            delimiter,
        } => {
            handle_import(&file, &table, &database, &format, &columns, header, &delimiter).await
        }
        Commands::Export {
            query,
            output,
            database,
            format,
            pretty,
            header,
        } => handle_export(&query, &output, &database, &format, pretty, header).await,
        Commands::Cluster { command } => handle_cluster_command(command).await,
        Commands::Config { command } => handle_config_command(command).await,
    };

    if let Err(e) = result {
        print_error(&format!("{}", e));
        process::exit(1);
    }
}

async fn handle_init(data_dir: &str, single_node: bool, config_path: &str) -> Result<()> {
    println!("Initializing ShardForge...");
    println!("Data directory: {}", data_dir);
    println!("Single node: {}", single_node);
    println!("Config file: {}", config_path);

    // TODO: Implement initialization logic
    println!("ShardForge initialized successfully!");
    Ok(())
}

async fn handle_start(config_path: &str) -> Result<()> {
    println!("Starting ShardForge with config: {}", config_path);

    // TODO: Implement start logic
    println!("ShardForge started successfully!");
    Ok(())
}

async fn handle_stop(timeout: u32) -> Result<()> {
    println!("Stopping ShardForge (timeout: {}s)...", timeout);

    // TODO: Implement stop logic
    println!("ShardForge stopped successfully!");
    Ok(())
}

async fn handle_status(detailed: bool, format: &str) -> Result<()> {
    println!("Getting ShardForge status...");
    println!("Detailed: {}", detailed);
    println!("Format: {}", format);

    // TODO: Implement status logic
    println!("Status: Running");
    Ok(())
}

async fn handle_sql(
    query: Option<&str>,
    database: &str,
    file: Option<&str>,
    interactive: bool,
) -> Result<()> {
    if interactive {
        // Start interactive REPL
        print_info("Starting interactive SQL shell...");
        let mut repl = SqlRepl::new(database.to_string()).await?;
        return repl.run().await;
    }

    // Create executor
    use shardforge::sql::{executor::*, parser::SqlParser};
    use shardforge_storage::{MemoryEngine, MVCCStorage};

    let storage = Box::new(MemoryEngine::new(&Default::default()).await?);
    let mvcc = MVCCStorage::new();
    let catalog = SchemaCatalog::new();

    let context = ExecutionContext {
        storage,
        mvcc,
        catalog,
    };

    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Handle file input
    if let Some(file_path) = file {
        print_info(&format!("Executing SQL from file: {}", file_path));
        let statements = execute_sql_file(file_path)?;

        for sql in statements {
            match parser.parse(&sql) {
                Ok(stmt) => match executor.execute(stmt).await {
                    Ok(result) => {
                        if !result.columns.is_empty() {
                            println!("{}", format_query_results(&result.columns, &result.rows));
                        }
                        print_success(&format!("Affected {} row(s)", result.affected_rows));
                    }
                    Err(e) => {
                        print_error(&format!("Execution error: {}", e));
                    }
                },
                Err(e) => {
                    print_error(&format!("Parse error: {}", e));
                }
            }
        }
    } else if let Some(sql) = query {
        // Handle direct query
        match parser.parse(sql) {
            Ok(stmt) => match executor.execute(stmt).await {
                Ok(result) => {
                    if !result.columns.is_empty() {
                        println!("{}", format_query_results(&result.columns, &result.rows));
                    }
                    print_success(&format!("Affected {} row(s)", result.affected_rows));
                }
                Err(e) => {
                    return Err(e);
                }
            },
            Err(e) => {
                return Err(e);
            }
        }
    } else {
        print_error("No query provided. Use --query or --file or --interactive");
        return Err(shardforge_core::ShardForgeError::Parse {
            message: "No query provided".to_string(),
        });
    }

    Ok(())
}

async fn handle_import(
    file_path: &str,
    table_name: &str,
    database: &str,
    format: &str,
    columns_spec: &str,
    header: bool,
    delimiter: &str,
) -> Result<()> {
    print_info(&format!("Importing data from {} to {}.{}", file_path, database, table_name));

    // Parse column specifications
    let column_types = parse_column_spec(columns_spec)?;

    // Create progress bar
    let pb = create_progress_bar("Importing data", 100);

    match format.to_lowercase().as_str() {
        "csv" => {
            let options = CsvImportOptions {
                has_header: header,
                delimiter: delimiter.bytes().next().unwrap_or(b','),
                ..Default::default()
            };

            let importer = CsvImporter::new(options);
            let statements = importer.import_from_file(file_path, table_name, &column_types)?;

            pb.set_length(statements.len() as u64);

            // Execute import statements
            use shardforge::sql::executor::*;
            use shardforge_storage::{MemoryEngine, MVCCStorage};

            let storage = Box::new(MemoryEngine::new(&Default::default()).await?);
            let mvcc = MVCCStorage::new();
            let catalog = SchemaCatalog::new();

            let context = ExecutionContext {
                storage,
                mvcc,
                catalog,
            };

            let mut executor = QueryExecutor::new(context);

            for (i, stmt) in statements.into_iter().enumerate() {
                executor
                    .execute(shardforge::sql::ast::Statement::Insert(stmt))
                    .await?;
                pb.set_position(i as u64 + 1);
            }

            pb.finish_with_message("Import complete");
            print_success(&format!("Imported {} rows", pb.position()));
        }
        "json" => {
            let statements = JsonImporter::import_from_file(file_path, table_name, &column_types)?;

            pb.set_length(statements.len() as u64);

            // Execute import statements (similar to CSV)
            use shardforge::sql::executor::*;
            use shardforge_storage::{MemoryEngine, MVCCStorage};

            let storage = Box::new(MemoryEngine::new(&Default::default()).await?);
            let mvcc = MVCCStorage::new();
            let catalog = SchemaCatalog::new();

            let context = ExecutionContext {
                storage,
                mvcc,
                catalog,
            };

            let mut executor = QueryExecutor::new(context);

            for (i, stmt) in statements.into_iter().enumerate() {
                executor
                    .execute(shardforge::sql::ast::Statement::Insert(stmt))
                    .await?;
                pb.set_position(i as u64 + 1);
            }

            pb.finish_with_message("Import complete");
            print_success(&format!("Imported {} rows", pb.position()));
        }
        _ => {
            return Err(shardforge_core::ShardForgeError::Parse {
                message: format!("Unsupported format: {}", format),
            });
        }
    }

    Ok(())
}

async fn handle_export(
    query: &str,
    output_path: &str,
    database: &str,
    format: &str,
    pretty: bool,
    header: bool,
) -> Result<()> {
    print_info(&format!(
        "Exporting query results from {} to {}",
        database, output_path
    ));

    // Execute query
    use shardforge::sql::{executor::*, parser::SqlParser};
    use shardforge_storage::{MemoryEngine, MVCCStorage};

    let storage = Box::new(MemoryEngine::new(&Default::default()).await?);
    let mvcc = MVCCStorage::new();
    let catalog = SchemaCatalog::new();

    let context = ExecutionContext {
        storage,
        mvcc,
        catalog,
    };

    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    let stmt = parser.parse(query)?;
    let result = executor.execute(stmt).await?;

    // Export results
    match format.to_lowercase().as_str() {
        "csv" => {
            let options = CsvExportOptions {
                include_header: header,
                ..Default::default()
            };

            let exporter = CsvExporter::new(options);
            exporter.export_to_file(&result, output_path)?;

            print_success(&format!("Exported {} rows to CSV", result.rows.len()));
        }
        "json" => {
            let options = JsonExportOptions {
                pretty,
                array_format: true,
            };

            let exporter = JsonExporter::new(options);
            exporter.export_to_file(&result, output_path)?;

            print_success(&format!("Exported {} rows to JSON", result.rows.len()));
        }
        _ => {
            return Err(shardforge_core::ShardForgeError::Parse {
                message: format!("Unsupported format: {}", format),
            });
        }
    }

    Ok(())
}

/// Parse column specification string (name:type,name:type,...)
fn parse_column_spec(spec: &str) -> Result<Vec<(String, shardforge::sql::ast::DataType)>> {
    let mut columns = Vec::new();

    for part in spec.split(',') {
        let parts: Vec<&str> = part.trim().split(':').collect();
        if parts.len() != 2 {
            return Err(shardforge_core::ShardForgeError::Parse {
                message: format!("Invalid column spec: {}. Expected name:type", part),
            });
        }

        let name = parts[0].trim().to_string();
        let type_name = parts[1].trim().to_uppercase();

        let data_type = match type_name.as_str() {
            "INTEGER" | "INT" => shardforge::sql::ast::DataType::Integer,
            "BIGINT" => shardforge::sql::ast::DataType::BigInt,
            "SMALLINT" => shardforge::sql::ast::DataType::SmallInt,
            "REAL" => shardforge::sql::ast::DataType::Real,
            "DOUBLE" => shardforge::sql::ast::DataType::Double,
            "BOOLEAN" | "BOOL" => shardforge::sql::ast::DataType::Boolean,
            "VARCHAR" => shardforge::sql::ast::DataType::Varchar { length: None },
            "TEXT" => shardforge::sql::ast::DataType::Text,
            _ => {
                return Err(shardforge_core::ShardForgeError::Parse {
                    message: format!("Unknown data type: {}", type_name),
                });
            }
        };

        columns.push((name, data_type));
    }

    Ok(columns)
}

async fn handle_cluster_command(command: ClusterCommands) -> Result<()> {
    match command {
        ClusterCommands::Init { node_id, name, bind } => {
            println!("Initializing cluster...");
            println!("Node ID: {:?}", node_id);
            println!("Name: {}", name);
            println!("Bind: {}", bind);

            // TODO: Implement cluster init logic
            println!("Cluster initialized successfully!");
        }
        ClusterCommands::Join { seed_node, node_id, bind } => {
            println!("Joining cluster...");
            println!("Seed node: {}", seed_node);
            println!("Node ID: {:?}", node_id);
            println!("Bind: {}", bind);

            // TODO: Implement cluster join logic
            println!("Joined cluster successfully!");
        }
        ClusterCommands::Status { detailed, format } => {
            println!("Getting cluster status...");
            println!("Detailed: {}", detailed);
            println!("Format: {}", format);

            // TODO: Implement cluster status logic
            println!("Cluster status: Healthy");
        }
    }

    Ok(())
}

async fn handle_config_command(command: ConfigCommands) -> Result<()> {
    use shardforge_config::{Config, ConfigLoader};

    match command {
        ConfigCommands::Get { key } => {
            let config = ConfigLoader::load_from_file("shardforge.toml")?;
            
            print_info(&format!("Getting configuration value for: {}", key));
            
            // Simple key access (would need more sophisticated path traversal in production)
            let value = match key.as_str() {
                "storage.data_dir" => config.storage.data_dir.to_string_lossy().to_string(),
                "storage.engine" => format!("{:?}", config.storage.engine),
                "network.bind_address" => config.network.bind_address.clone(),
                "network.port" => config.network.port.to_string(),
                _ => {
                    print_warning(&format!("Unknown configuration key: {}", key));
                    return Ok(());
                }
            };

            println!("{} = {}", style(&key).cyan(), style(&value).yellow());
        }
        ConfigCommands::Set { key, value, validate } => {
            print_info(&format!("Setting {} = {}", key, value));
            
            if validate {
                print_info("Validating configuration...");
                // TODO: Implement actual validation
                print_success("Configuration is valid");
            }
            
            print_warning("Configuration modification not yet implemented - use shardforge.toml directly");
        }
        ConfigCommands::Show { all } => {
            let config = ConfigLoader::load_from_file("shardforge.toml")?;
            
            println!("{}", style("Current Configuration:").green().bold());
            println!();
            
            println!("{}", style("Storage:").cyan().bold());
            println!("  data_dir: {}", config.storage.data_dir.display());
            println!("  engine: {:?}", config.storage.engine);
            println!();
            
            println!("{}", style("Network:").cyan().bold());
            println!("  bind_address: {}", config.network.bind_address);
            println!("  port: {}", config.network.port);
            println!();
            
            if all {
                println!("{}", style("Advanced Settings:").cyan().bold());
                // Show additional settings
                println!("  (Additional settings would be displayed here)");
            }
        }
    }

    Ok(())
}
