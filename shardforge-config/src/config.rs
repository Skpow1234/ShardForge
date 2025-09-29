//! Configuration structures for ShardForge

use serde::{Deserialize, Serialize};
use shardforge_core::*;
use std::path::PathBuf;

/// Main configuration structure for ShardForge
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardForgeConfig {
    /// Cluster configuration
    pub cluster: ClusterSection,
    /// Node configuration
    pub node: NodeSection,
    /// Storage configuration
    pub storage: StorageSection,
    /// Network configuration
    pub network: NetworkSection,
    /// Logging configuration
    pub logging: LoggingSection,
    /// Metrics and monitoring configuration
    pub monitoring: MonitoringSection,
}

/// Cluster-related configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterSection {
    /// Cluster name
    pub name: String,
    /// Node ID (auto-generated if not provided)
    pub node_id: Option<String>,
    /// Data directory
    pub data_directory: PathBuf,
    /// Cluster members (for joining existing cluster)
    #[serde(default)]
    pub members: Vec<String>,
}

/// Node-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeSection {
    /// Node name
    pub name: Option<String>,
    /// Bind address for client connections
    pub bind_address: String,
    /// Advertised address (for NAT/firewall scenarios)
    pub advertised_address: Option<String>,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout_sec: u32,
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageSection {
    /// Storage engine type
    pub engine: StorageEngineType,
    /// Block cache size in MB
    pub block_cache_size_mb: usize,
    /// Write buffer size in MB
    pub write_buffer_size_mb: usize,
    /// Maximum write buffer number
    pub max_write_buffer_number: usize,
    /// Compression algorithm
    pub compression: CompressionType,
    /// Enable statistics
    pub enable_statistics: bool,
}

/// Network configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSection {
    /// TLS enabled
    pub tls_enabled: bool,
    /// Certificate path
    pub certificate_path: Option<PathBuf>,
    /// Private key path
    pub private_key_path: Option<PathBuf>,
    /// CA certificate path
    pub ca_certificate_path: Option<PathBuf>,
    /// Maximum message size in MB
    pub max_message_size_mb: usize,
    /// Connection pool size
    pub connection_pool_size: usize,
    /// Keep alive interval in seconds
    pub keep_alive_interval_sec: u32,
}

/// Logging configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingSection {
    /// Log level
    pub level: LogLevel,
    /// Log format
    pub format: LogFormat,
    /// Log file path (optional, stdout if not specified)
    pub file_path: Option<PathBuf>,
    /// Maximum log file size in MB
    pub max_file_size_mb: usize,
    /// Maximum number of log files to keep
    pub max_files: usize,
}

/// Monitoring configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringSection {
    /// Enable metrics collection
    pub enabled: bool,
    /// Metrics server bind address
    pub bind_address: String,
    /// Enable tracing
    pub tracing_enabled: bool,
    /// Tracing service name
    pub service_name: String,
}

/// Storage engine types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum StorageEngineType {
    /// RocksDB (recommended for production)
    RocksDB,
    /// Sled (pure Rust alternative)
    Sled,
    /// In-memory (for testing)
    Memory,
}

impl Default for StorageEngineType {
    fn default() -> Self {
        Self::RocksDB
    }
}

/// Compression algorithms
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionType {
    /// No compression
    None,
    /// Snappy compression (fast)
    Snappy,
    /// LZ4 compression (balanced)
    Lz4,
    /// Zstandard compression (high compression)
    Zstd,
}

impl Default for CompressionType {
    fn default() -> Self {
        Self::Lz4
    }
}

/// Log levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogLevel {
    /// Trace level
    Trace,
    /// Debug level
    Debug,
    /// Info level
    Info,
    /// Warning level
    Warn,
    /// Error level
    Error,
}

impl Default for LogLevel {
    fn default() -> Self {
        Self::Info
    }
}

/// Log formats
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogFormat {
    /// JSON format (structured)
    Json,
    /// Human-readable format
    Pretty,
}

impl Default for LogFormat {
    fn default() -> Self {
        Self::Pretty
    }
}

impl Default for ShardForgeConfig {
    fn default() -> Self {
        Self {
            cluster: ClusterSection::default(),
            node: NodeSection::default(),
            storage: StorageSection::default(),
            network: NetworkSection::default(),
            logging: LoggingSection::default(),
            monitoring: MonitoringSection::default(),
        }
    }
}

impl Default for ClusterSection {
    fn default() -> Self {
        Self {
            name: "default-cluster".to_string(),
            node_id: None,
            data_directory: PathBuf::from("./data"),
            members: Vec::new(),
        }
    }
}

impl Default for NodeSection {
    fn default() -> Self {
        Self {
            name: None,
            bind_address: "127.0.0.1:5432".to_string(),
            advertised_address: None,
            max_connections: 1000,
            connection_timeout_sec: 30,
        }
    }
}

impl Default for StorageSection {
    fn default() -> Self {
        Self {
            engine: StorageEngineType::default(),
            block_cache_size_mb: 256,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 4,
            compression: CompressionType::default(),
            enable_statistics: true,
        }
    }
}

impl Default for NetworkSection {
    fn default() -> Self {
        Self {
            tls_enabled: false,
            certificate_path: None,
            private_key_path: None,
            ca_certificate_path: None,
            max_message_size_mb: 4,
            connection_pool_size: 10,
            keep_alive_interval_sec: 60,
        }
    }
}

impl Default for LoggingSection {
    fn default() -> Self {
        Self {
            level: LogLevel::default(),
            format: LogFormat::default(),
            file_path: None,
            max_file_size_mb: 100,
            max_files: 5,
        }
    }
}

impl Default for MonitoringSection {
    fn default() -> Self {
        Self {
            enabled: true,
            bind_address: "127.0.0.1:9090".to_string(),
            tracing_enabled: false,
            service_name: "shardforge".to_string(),
        }
    }
}
