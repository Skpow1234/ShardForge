//! Cluster-related types and interfaces

use crate::{NodeId, NodeInfo, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

/// Unique identifier for a cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ClusterId(Uuid);

impl ClusterId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for ClusterId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for ClusterId {
    fn default() -> Self {
        Self::new()
    }
}

use std::fmt;

/// Information about a cluster
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfo {
    /// Unique cluster identifier
    pub id: ClusterId,
    /// Human-readable cluster name
    pub name: String,
    /// Cluster creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Current cluster version
    pub version: String,
    /// Cluster configuration
    pub config: ClusterConfig,
    /// Member nodes
    pub members: HashMap<NodeId, NodeInfo>,
}

/// Cluster-wide configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterConfig {
    /// Replication factor (number of replicas per shard)
    pub replication_factor: u32,
    /// Number of shards
    pub shard_count: u32,
    /// Consistency level
    pub consistency_level: ConsistencyLevel,
    /// Cluster timeout settings
    pub timeouts: ClusterTimeouts,
    /// Network settings
    pub network: ClusterNetwork,
}

/// Consistency levels for operations
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsistencyLevel {
    /// Strong consistency (wait for majority)
    Strong,
    /// Eventual consistency (fire and forget)
    Eventual,
    /// Read-your-writes consistency
    ReadYourWrites,
    /// Session consistency
    Session,
}

impl fmt::Display for ConsistencyLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConsistencyLevel::Strong => write!(f, "strong"),
            ConsistencyLevel::Eventual => write!(f, "eventual"),
            ConsistencyLevel::ReadYourWrites => write!(f, "read_your_writes"),
            ConsistencyLevel::Session => write!(f, "session"),
        }
    }
}

/// Cluster timeout settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTimeouts {
    /// Heartbeat interval between nodes
    pub heartbeat_interval_ms: u64,
    /// Election timeout (minimum)
    pub election_timeout_min_ms: u64,
    /// Election timeout (maximum)
    pub election_timeout_max_ms: u64,
    /// RPC timeout
    pub rpc_timeout_ms: u64,
    /// Leader lease timeout
    pub leader_lease_timeout_ms: u64,
}

impl Default for ClusterTimeouts {
    fn default() -> Self {
        Self {
            heartbeat_interval_ms: 50,
            election_timeout_min_ms: 150,
            election_timeout_max_ms: 300,
            rpc_timeout_ms: 5000,
            leader_lease_timeout_ms: 1000,
        }
    }
}

/// Cluster network settings
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterNetwork {
    /// Bind address for cluster communication
    pub bind_address: String,
    /// Advertised address for cluster communication
    pub advertised_address: Option<String>,
    /// TLS enabled for cluster communication
    pub tls_enabled: bool,
    /// Maximum message size
    pub max_message_size: usize,
    /// Connection pool size
    pub connection_pool_size: usize,
}

impl Default for ClusterNetwork {
    fn default() -> Self {
        Self {
            bind_address: "0.0.0.0:5432".to_string(),
            advertised_address: None,
            tls_enabled: false,
            max_message_size: 4 * 1024 * 1024, // 4MB
            connection_pool_size: 10,
        }
    }
}

/// Cluster topology information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterTopology {
    /// Cluster information
    pub cluster: ClusterInfo,
    /// Leader node ID (if any)
    pub leader: Option<NodeId>,
    /// Term number
    pub term: u64,
    /// Last log index
    pub last_log_index: u64,
    /// Last log term
    pub last_log_term: u64,
}

/// Cluster membership event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MembershipEvent {
    /// Node joined the cluster
    NodeJoined {
        node_id: NodeId,
        node_info: NodeInfo,
    },
    /// Node left the cluster
    NodeLeft { node_id: NodeId },
    /// Node failed and was removed
    NodeFailed { node_id: NodeId, reason: String },
    /// Leadership changed
    LeadershipChanged {
        old_leader: Option<NodeId>,
        new_leader: NodeId,
        term: u64,
    },
}

/// Cluster status information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterStatus {
    /// Overall cluster health
    pub health: ClusterHealth,
    /// Number of nodes
    pub node_count: usize,
    /// Number of healthy nodes
    pub healthy_nodes: usize,
    /// Number of unhealthy nodes
    pub unhealthy_nodes: usize,
    /// Current leader
    pub leader: Option<NodeId>,
    /// Cluster uptime
    pub uptime: std::time::Duration,
    /// Recent events
    pub recent_events: Vec<MembershipEvent>,
}

/// Cluster health status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ClusterHealth {
    /// All nodes healthy
    Healthy,
    /// Some nodes degraded but cluster operational
    Degraded,
    /// Critical issues affecting cluster operation
    Unhealthy,
    /// Cluster unavailable
    Critical,
}

impl fmt::Display for ClusterHealth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusterHealth::Healthy => write!(f, "healthy"),
            ClusterHealth::Degraded => write!(f, "degraded"),
            ClusterHealth::Unhealthy => write!(f, "unhealthy"),
            ClusterHealth::Critical => write!(f, "critical"),
        }
    }
}

impl ClusterInfo {
    pub fn new(name: String, config: ClusterConfig) -> Self {
        Self {
            id: ClusterId::new(),
            name,
            created_at: chrono::Utc::now(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            config,
            members: HashMap::new(),
        }
    }

    pub fn add_node(&mut self, node_info: NodeInfo) -> Result<()> {
        if self.members.contains_key(&node_info.id) {
            return Err(crate::ShardForgeError::internal(format!(
                "Node {} already exists in cluster",
                node_info.id
            )));
        }
        self.members.insert(node_info.id, node_info);
        Ok(())
    }

    pub fn remove_node(&mut self, node_id: &NodeId) -> Result<NodeInfo> {
        self.members.remove(node_id).ok_or_else(|| {
            crate::ShardForgeError::internal(format!("Node {} not found in cluster", node_id))
        })
    }

    pub fn get_node(&self, node_id: &NodeId) -> Option<&NodeInfo> {
        self.members.get(node_id)
    }

    pub fn get_nodes_by_role(&self, role: crate::NodeRole) -> Vec<&NodeInfo> {
        self.members.values().filter(|node| node.role == role).collect()
    }

    pub fn get_leader(&self) -> Option<&NodeInfo> {
        self.members.values().find(|node| node.is_leader())
    }

    pub fn node_count(&self) -> usize {
        self.members.len()
    }

    pub fn quorum_size(&self) -> usize {
        (self.node_count() / 2) + 1
    }
}

impl Default for ClusterConfig {
    fn default() -> Self {
        Self {
            replication_factor: 3,
            shard_count: 16,
            consistency_level: ConsistencyLevel::Strong,
            timeouts: ClusterTimeouts::default(),
            network: ClusterNetwork::default(),
        }
    }
}
