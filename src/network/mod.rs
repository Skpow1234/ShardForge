//! Network communication layer for ShardForge
//!
//! This module provides the gRPC server implementation and client connectivity
//! for distributed database operations.

pub mod client;
pub mod connection;
pub mod protocol;
pub mod server;

// Re-export main types
pub use client::*;
pub use connection::*;
pub use protocol::*;
pub use server::*;

/// Network configuration
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Server bind address
    pub bind_address: String,
    /// Maximum concurrent connections
    pub max_connections: u32,
    /// Connection timeout in seconds
    pub connection_timeout_sec: u32,
    /// Keep-alive interval in seconds
    pub keep_alive_interval_sec: u32,
    /// Maximum message size in bytes
    pub max_message_size_bytes: u32,
    /// Enable compression
    pub enable_compression: bool,
    /// Compression level (1-9)
    pub compression_level: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:5432".to_string(),
            max_connections: 1000,
            connection_timeout_sec: 30,
            keep_alive_interval_sec: 60,
            max_message_size_bytes: 4 * 1024 * 1024, // 4MB
            enable_compression: true,
            compression_level: 6,
        }
    }
}
