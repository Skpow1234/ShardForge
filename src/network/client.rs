//! gRPC client for connecting to other ShardForge nodes

// use std::time::Duration; // TODO: Remove when implementing timeout functionality

use shardforge_core::{NodeId, Result, ShardForgeError};
use tonic::transport::Channel;

use crate::network::protocol::proto;
use crate::network::{ConnectionPool, NetworkConfig};

/// gRPC client for ShardForge database
pub struct DatabaseClient {
    pool: ConnectionPool,
    client: proto::database_service_client::DatabaseServiceClient<Channel>,
}

impl DatabaseClient {
    /// Create a new database client
    pub async fn new(config: NetworkConfig, node_id: &NodeId, address: &str) -> Result<Self> {
        let pool = ConnectionPool::new(config);
        let channel = pool.get_connection(node_id, address).await?;
        let client = proto::database_service_client::DatabaseServiceClient::new(channel);

        Ok(Self { pool, client })
    }

    /// Execute a query
    pub async fn execute_query(
        &mut self,
        query_id: String,
        sql: String,
        parameters: Vec<proto::Parameter>,
    ) -> Result<proto::QueryResponse> {
        let request = proto::QueryRequest {
            query_id,
            sql,
            parameters,
            options: None,
            streaming_response: false,
        };

        let response =
            self.client.execute_query(request).await.map_err(|e| ShardForgeError::Network {
                message: format!("Query execution failed: {}", e),
            })?;

        Ok(response.into_inner())
    }

    /// Begin a transaction
    pub async fn begin_transaction(
        &mut self,
        isolation_level: proto::IsolationLevel,
        timeout_seconds: u32,
        read_only: bool,
    ) -> Result<proto::TransactionResponse> {
        let request = proto::TransactionRequest {
            isolation_level: isolation_level as i32,
            timeout_seconds,
            read_only,
        };

        let response =
            self.client
                .begin_transaction(request)
                .await
                .map_err(|e| ShardForgeError::Network {
                    message: format!("Transaction begin failed: {}", e),
                })?;

        Ok(response.into_inner())
    }

    /// Commit a transaction
    pub async fn commit_transaction(
        &mut self,
        transaction_id: String,
        _commit_timestamp: u64,
    ) -> Result<proto::CommitResponse> {
        let request = proto::CommitRequest { transaction_id };

        let response = self.client.commit_transaction(request).await.map_err(|e| {
            ShardForgeError::Network {
                message: format!("Transaction commit failed: {}", e),
            }
        })?;

        Ok(response.into_inner())
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(
        &mut self,
        transaction_id: String,
        _reason: String,
    ) -> Result<proto::RollbackResponse> {
        let request = proto::RollbackRequest { transaction_id };

        let response = self.client.rollback_transaction(request).await.map_err(|e| {
            ShardForgeError::Network {
                message: format!("Transaction rollback failed: {}", e),
            }
        })?;

        Ok(response.into_inner())
    }

    /// Get cluster status
    pub async fn get_cluster_status(
        &mut self,
        include_metrics: bool,
    ) -> Result<proto::StatusResponse> {
        let request = proto::StatusRequest { include_metrics };

        let response = self.client.get_cluster_status(request).await.map_err(|e| {
            ShardForgeError::Network {
                message: format!("Cluster status request failed: {}", e),
            }
        })?;

        Ok(response.into_inner())
    }

    /// Send heartbeat
    pub async fn heartbeat(
        &mut self,
        node_id: String,
        _timestamp: u64,
        status: proto::NodeStatus,
    ) -> Result<proto::HeartbeatResponse> {
        let request = proto::HeartbeatRequest { node_id, status };

        let response =
            self.client.heartbeat(request).await.map_err(|e| ShardForgeError::Network {
                message: format!("Heartbeat failed: {}", e),
            })?;

        Ok(response.into_inner())
    }

    /// Get cluster topology
    pub async fn get_cluster_topology(
        &mut self,
        include_stats: bool,
    ) -> Result<proto::TopologyResponse> {
        let request = proto::TopologyRequest { include_stats };

        let response = self.client.get_cluster_topology(request).await.map_err(|e| {
            ShardForgeError::Network {
                message: format!("Topology request failed: {}", e),
            }
        })?;

        Ok(response.into_inner())
    }
}

/// Client factory for creating database clients
pub struct ClientFactory {
    config: NetworkConfig,
    pool: ConnectionPool,
}

impl ClientFactory {
    /// Create a new client factory
    pub fn new(config: NetworkConfig) -> Self {
        let pool = ConnectionPool::new(config.clone());
        Self { config, pool }
    }

    /// Create a client for a specific node
    pub async fn create_client(&self, node_id: &NodeId, address: &str) -> Result<DatabaseClient> {
        DatabaseClient::new(self.config.clone(), node_id, address).await
    }

    /// Get connection pool statistics
    pub async fn connection_count(&self) -> usize {
        self.pool.connection_count().await
    }

    /// Close all connections
    pub async fn close_all(&self) {
        self.pool.close_all().await;
    }
}
