//! RAFT Consensus Implementation
//!
//! This module implements the RAFT consensus algorithm for distributed consensus
//! in ShardForge. It provides strong consistency guarantees through leader election,
//! log replication, and state machine replication.

pub mod config;
pub mod log;
pub mod node;
pub mod rpc;
pub mod snapshot;
pub mod state;
pub mod storage;

pub use config::*;
pub use log::*;
pub use node::*;
pub use rpc::*;
pub use snapshot::*;
pub use state::*;
pub use storage::*;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Node ID in the RAFT cluster
pub type NodeId = u64;

/// Term number in RAFT protocol
pub type Term = u64;

/// Log index
pub type LogIndex = u64;

/// RAFT node role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Role {
    /// Follower - accepts log entries from leader
    Follower,
    /// Candidate - attempting to become leader
    Candidate,
    /// Leader - coordinates log replication
    Leader,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Role::Follower => write!(f, "Follower"),
            Role::Candidate => write!(f, "Candidate"),
            Role::Leader => write!(f, "Leader"),
        }
    }
}

/// RAFT error types
#[derive(Debug, thiserror::Error)]
pub enum RaftError {
    #[error("Not leader (current leader: {0:?})")]
    NotLeader(Option<NodeId>),

    #[error("Node not found: {0}")]
    NodeNotFound(NodeId),

    #[error("Invalid term: {0}")]
    InvalidTerm(Term),

    #[error("Log inconsistency at index {0}")]
    LogInconsistency(LogIndex),

    #[error("Snapshot error: {0}")]
    SnapshotError(String),

    #[error("Storage error: {0}")]
    StorageError(String),

    #[error("RPC error: {0}")]
    RpcError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Timeout: {0}")]
    Timeout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, RaftError>;

