//! Node-related types and interfaces

use std::net::SocketAddr;

use serde::{Deserialize, Serialize};

use crate::NodeId;

/// Represents the state of a database node
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeState {
    /// Node is initializing
    Initializing,
    /// Node is running normally
    Running,
    /// Node is shutting down
    ShuttingDown,
    /// Node has shut down
    Shutdown,
    /// Node is in maintenance mode
    Maintenance,
    /// Node has failed
    Failed,
}

impl fmt::Display for NodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeState::Initializing => write!(f, "initializing"),
            NodeState::Running => write!(f, "running"),
            NodeState::ShuttingDown => write!(f, "shutting_down"),
            NodeState::Shutdown => write!(f, "shutdown"),
            NodeState::Maintenance => write!(f, "maintenance"),
            NodeState::Failed => write!(f, "failed"),
        }
    }
}

use std::fmt;

/// Role of a node in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NodeRole {
    /// Leader node (handles writes and coordinates)
    Leader,
    /// Follower node (replicates from leader)
    Follower,
    /// Candidate node (running for election)
    Candidate,
    /// Observer node (read-only, no voting rights)
    Observer,
    /// Witness node (voting rights, no data storage)
    Witness,
}

impl fmt::Display for NodeRole {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeRole::Leader => write!(f, "leader"),
            NodeRole::Follower => write!(f, "follower"),
            NodeRole::Candidate => write!(f, "candidate"),
            NodeRole::Observer => write!(f, "observer"),
            NodeRole::Witness => write!(f, "witness"),
        }
    }
}

/// Information about a cluster node
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeInfo {
    /// Unique node identifier
    pub id: NodeId,
    /// Human-readable node name
    pub name: String,
    /// Node's network address
    pub address: SocketAddr,
    /// Node's advertised address (for NAT/firewall scenarios)
    pub advertised_address: Option<SocketAddr>,
    /// Current node state
    pub state: NodeState,
    /// Current node role in the cluster
    pub role: NodeRole,
    /// When the node joined the cluster
    pub joined_at: chrono::DateTime<chrono::Utc>,
    /// Node version information
    pub version: String,
    /// Node capabilities
    pub capabilities: Vec<NodeCapability>,
}

/// Node capabilities for feature detection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum NodeCapability {
    /// Supports RAFT consensus
    RaftConsensus,
    /// Supports SQL queries
    SqlQueries,
    /// Supports transactions
    Transactions,
    /// Supports encryption
    Encryption,
    /// Supports compression
    Compression,
    /// Supports backup operations
    Backup,
    /// Supports monitoring/metrics
    Monitoring,
}

impl fmt::Display for NodeCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NodeCapability::RaftConsensus => write!(f, "raft_consensus"),
            NodeCapability::SqlQueries => write!(f, "sql_queries"),
            NodeCapability::Transactions => write!(f, "transactions"),
            NodeCapability::Encryption => write!(f, "encryption"),
            NodeCapability::Compression => write!(f, "compression"),
            NodeCapability::Backup => write!(f, "backup"),
            NodeCapability::Monitoring => write!(f, "monitoring"),
        }
    }
}

/// Node health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeHealth {
    /// Overall health status
    pub status: HealthStatus,
    /// Detailed health checks
    pub checks: Vec<HealthCheck>,
    /// Last health check timestamp
    pub last_check: chrono::DateTime<chrono::Utc>,
}

/// Health status levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HealthStatus {
    /// Node is healthy
    Healthy,
    /// Node has minor issues but is operational
    Degraded,
    /// Node has critical issues
    Unhealthy,
    /// Node is unreachable or failed
    Critical,
}

impl fmt::Display for HealthStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            HealthStatus::Healthy => write!(f, "healthy"),
            HealthStatus::Degraded => write!(f, "degraded"),
            HealthStatus::Unhealthy => write!(f, "unhealthy"),
            HealthStatus::Critical => write!(f, "critical"),
        }
    }
}

/// Individual health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheck {
    /// Check name
    pub name: String,
    /// Check status
    pub status: HealthStatus,
    /// Check message
    pub message: String,
    /// Check duration
    pub duration: std::time::Duration,
    /// Check timestamp
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl NodeInfo {
    pub fn new(id: NodeId, name: String, address: SocketAddr) -> Self {
        Self {
            id,
            name,
            address,
            advertised_address: None,
            state: NodeState::Initializing,
            role: NodeRole::Follower,
            joined_at: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            capabilities: vec![
                NodeCapability::SqlQueries,
                NodeCapability::Transactions,
                NodeCapability::Monitoring,
            ],
        }
    }

    pub fn with_advertised_address(mut self, addr: SocketAddr) -> Self {
        self.advertised_address = Some(addr);
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<NodeCapability>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn effective_address(&self) -> SocketAddr {
        self.advertised_address.unwrap_or(self.address)
    }

    pub fn has_capability(&self, capability: NodeCapability) -> bool {
        self.capabilities.contains(&capability)
    }

    pub fn is_leader(&self) -> bool {
        matches!(self.role, NodeRole::Leader)
    }

    pub fn can_vote(&self) -> bool {
        !matches!(self.role, NodeRole::Observer)
    }
}
