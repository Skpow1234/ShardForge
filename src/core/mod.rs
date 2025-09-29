//! Core types and utilities for ShardForge

// Core modules (to be implemented)
// pub mod error;
// pub mod types;
// pub mod node;
// pub mod cluster;

// Re-export commonly used types (when modules are implemented)
// pub use error::*;
// pub use types::*;
// pub use node::*;
// pub use cluster::*;

// Temporary types for CLI functionality
use std::result::Result as StdResult;

/// Result type alias for ShardForge operations
pub type Result<T> = StdResult<T, ShardForgeError>;

/// Basic error type for ShardForge
#[derive(Debug, Clone)]
pub enum ShardForgeError {
    /// Internal error with message
    Internal(String),
    /// Configuration error
    Config(String),
    /// Storage error
    Storage(String),
    /// Network error
    Network(String),
    /// Consensus error
    Consensus(String),
}

impl std::fmt::Display for ShardForgeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ShardForgeError::Internal(msg) => write!(f, "Internal error: {}", msg),
            ShardForgeError::Config(msg) => write!(f, "Configuration error: {}", msg),
            ShardForgeError::Storage(msg) => write!(f, "Storage error: {}", msg),
            ShardForgeError::Network(msg) => write!(f, "Network error: {}", msg),
            ShardForgeError::Consensus(msg) => write!(f, "Consensus error: {}", msg),
        }
    }
}

impl std::error::Error for ShardForgeError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_types() {
        let config_err = ShardForgeError::config("test config error");
        match config_err {
            ShardForgeError::Config(msg) => assert_eq!(msg, "test config error"),
            _ => panic!("Wrong error variant"),
        }

        let storage_err = ShardForgeError::storage(std::io::Error::new(std::io::ErrorKind::NotFound, "file not found"));
        match storage_err {
            ShardForgeError::Storage(_) => {},
            _ => panic!("Wrong error variant"),
        }
    }

    #[test]
    fn test_node_id_generation() {
        let id1 = NodeId::new();
        let id2 = NodeId::new();
        assert_ne!(id1, id2);

        let id_str = id1.to_string();
        let parsed = NodeId::from_string(&id_str).unwrap();
        assert_eq!(id1, parsed);
    }

    #[test]
    fn test_cluster_id_generation() {
        let id1 = ClusterId::new();
        let id2 = ClusterId::new();
        assert_ne!(id1, id2);

        let id_str = id1.to_string();
        let parsed = ClusterId::from_string(&id_str).unwrap();
        assert_eq!(id1, parsed);
    }

    #[test]
    fn test_key_operations() {
        let key = Key::from_string("test_key");
        assert_eq!(key.len(), 8);
        assert!(!key.is_empty());

        let empty_key = Key::from_slice(&[]);
        assert!(empty_key.is_empty());
    }

    #[test]
    fn test_value_operations() {
        let value = Value::from_string("test_value");
        assert_eq!(value.len(), 10);
        assert!(!value.is_empty());

        let empty_value = Value::from_slice(&[]);
        assert!(empty_value.is_empty());
    }

    #[test]
    fn test_version_operations() {
        let v1 = Version::new(1);
        let v2 = Version::new(2);
        assert!(v1 < v2);

        let v3 = v1.increment();
        assert_eq!(v3.value(), 2);
    }

    #[test]
    fn test_timestamp_operations() {
        let ts1 = Timestamp::new(1000);
        let ts2 = Timestamp::new(2000);
        assert!(ts1 < ts2);

        let now = Timestamp::now();
        assert!(now.value() > 0);
    }

    #[test]
    fn test_node_info_operations() {
        use std::net::SocketAddr;

        let addr: SocketAddr = "127.0.0.1:5432".parse().unwrap();
        let node = NodeInfo::new(NodeId::new(), "test-node".to_string(), addr);

        assert_eq!(node.name, "test-node");
        assert_eq!(node.address, addr);
        assert_eq!(node.state, NodeState::Initializing);
        assert_eq!(node.role, NodeRole::Follower);
        assert!(node.capabilities.contains(&NodeCapability::SqlQueries));
        assert!(node.can_vote());
        assert!(!node.is_leader());
    }

    #[test]
    fn test_cluster_info_operations() {
        use crate::config::ClusterConfig;

        let config = ClusterConfig::default();
        let mut cluster = ClusterInfo::new("test-cluster".to_string(), config);

        let node_id = NodeId::new();
        let addr: std::net::SocketAddr = "127.0.0.1:5432".parse().unwrap();
        let node_info = NodeInfo::new(node_id.clone(), "node1".to_string(), addr);

        cluster.add_node(node_info.clone()).unwrap();
        assert_eq!(cluster.node_count(), 1);

        let retrieved = cluster.get_node(&node_id).unwrap();
        assert_eq!(retrieved.id, node_id);
        assert_eq!(retrieved.name, "node1");

        let removed = cluster.remove_node(&node_id).unwrap();
        assert_eq!(removed.id, node_id);
        assert_eq!(cluster.node_count(), 0);
    }
}
