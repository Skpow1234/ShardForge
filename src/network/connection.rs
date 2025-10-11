//! Connection management and pooling

use std::sync::Arc;
use std::time::Duration;

use shardforge_core::{NodeId, Result, ShardForgeError};
use tokio::sync::RwLock;
use tonic::transport::{Channel, Endpoint};

use crate::network::NetworkConfig;

/// Connection pool for managing gRPC connections
pub struct ConnectionPool {
    config: NetworkConfig,
    connections: Arc<RwLock<std::collections::HashMap<NodeId, Channel>>>,
}

impl ConnectionPool {
    /// Create a new connection pool
    pub fn new(config: NetworkConfig) -> Self {
        Self {
            config,
            connections: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Get or create a connection to a node
    pub async fn get_connection(&self, node_id: &NodeId, address: &str) -> Result<Channel> {
        // Check if we already have a connection
        {
            let connections = self.connections.read().await;
            if let Some(channel) = connections.get(node_id) {
                return Ok(channel.clone());
            }
        }

        // Create new connection
        let endpoint = Endpoint::from_shared(address.to_string())
            .map_err(|e| ShardForgeError::Network {
                message: format!("Invalid endpoint address {}: {}", address, e),
            })?
            .timeout(Duration::from_secs(self.config.connection_timeout_sec as u64))
            .keep_alive_timeout(Duration::from_secs(self.config.keep_alive_interval_sec as u64));

        let channel = endpoint.connect().await.map_err(|e| ShardForgeError::Network {
            message: format!("Failed to connect to {}: {}", address, e),
        })?;

        // Store the connection
        {
            let mut connections = self.connections.write().await;
            connections.insert(node_id.clone(), channel.clone());
        }

        Ok(channel)
    }

    /// Remove a connection from the pool
    pub async fn remove_connection(&self, node_id: &NodeId) {
        let mut connections = self.connections.write().await;
        connections.remove(node_id);
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        let connections = self.connections.read().await;
        connections.len()
    }

    /// Close all connections
    pub async fn close_all(&self) {
        let mut connections = self.connections.write().await;
        connections.clear();
    }
}

/// Load balancer for distributing requests across nodes
pub struct LoadBalancer {
    nodes: Arc<RwLock<Vec<NodeId>>>,
    current_index: Arc<RwLock<usize>>,
}

impl LoadBalancer {
    /// Create a new load balancer
    pub fn new() -> Self {
        Self {
            nodes: Arc::new(RwLock::new(Vec::new())),
            current_index: Arc::new(RwLock::new(0)),
        }
    }

    /// Add a node to the load balancer
    pub async fn add_node(&self, node_id: NodeId) {
        let mut nodes = self.nodes.write().await;
        if !nodes.contains(&node_id) {
            nodes.push(node_id);
        }
    }

    /// Remove a node from the load balancer
    pub async fn remove_node(&self, node_id: &NodeId) {
        let mut nodes = self.nodes.write().await;
        nodes.retain(|n| n != node_id);
    }

    /// Get the next node using round-robin strategy
    pub async fn next_node(&self) -> Option<NodeId> {
        let nodes = self.nodes.read().await;
        if nodes.is_empty() {
            return None;
        }

        let mut current_index = self.current_index.write().await;
        let node = nodes[*current_index].clone();
        *current_index = (*current_index + 1) % nodes.len();

        Some(node)
    }

    /// Get all available nodes
    pub async fn get_nodes(&self) -> Vec<NodeId> {
        let nodes = self.nodes.read().await;
        nodes.clone()
    }
}

/// Circuit breaker for fault tolerance
pub struct CircuitBreaker {
    failure_count: Arc<RwLock<u32>>,
    failure_threshold: u32,
    recovery_timeout: Duration,
    last_failure_time: Arc<RwLock<Option<std::time::Instant>>>,
    state: Arc<RwLock<CircuitState>>,
}

#[derive(Debug, Clone, PartialEq)]
/// Circuit breaker state for connection management
pub enum CircuitState {
    /// Circuit is closed, requests are allowed
    Closed,
    /// Circuit is open, requests are blocked
    Open,
    /// Circuit is half-open, testing if service is back
    HalfOpen,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(failure_threshold: u32, recovery_timeout: Duration) -> Self {
        Self {
            failure_count: Arc::new(RwLock::new(0)),
            failure_threshold,
            recovery_timeout,
            last_failure_time: Arc::new(RwLock::new(None)),
            state: Arc::new(RwLock::new(CircuitState::Closed)),
        }
    }

    /// Check if the circuit breaker allows the operation
    pub async fn can_execute(&self) -> bool {
        let state = self.state.read().await;
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if enough time has passed to try again
                let last_failure = self.last_failure_time.read().await;
                if let Some(last_failure_time) = *last_failure {
                    if last_failure_time.elapsed() > self.recovery_timeout {
                        drop(last_failure);
                        drop(state);
                        *self.state.write().await = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            }
            CircuitState::HalfOpen => true,
        }
    }

    /// Record a successful operation
    pub async fn record_success(&self) {
        *self.failure_count.write().await = 0;
        *self.state.write().await = CircuitState::Closed;
    }

    /// Record a failed operation
    pub async fn record_failure(&self) {
        let mut failure_count = self.failure_count.write().await;
        *failure_count += 1;

        *self.last_failure_time.write().await = Some(std::time::Instant::now());

        if *failure_count >= self.failure_threshold {
            *self.state.write().await = CircuitState::Open;
        }
    }

    /// Get current state
    pub async fn get_state(&self) -> CircuitState {
        self.state.read().await.clone()
    }
}
