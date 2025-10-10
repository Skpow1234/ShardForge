//! gRPC server implementation

use std::net::SocketAddr;
use std::sync::Arc;

use async_trait::async_trait;
use shardforge_core::{Result, ShardForgeError};
use tokio::sync::RwLock;
use tonic::{transport::Server, Request, Response, Status};

use crate::network::{NetworkConfig, DatabaseServiceImpl};

/// gRPC server for ShardForge database
pub struct GrpcServer {
    config: NetworkConfig,
    service: Arc<DatabaseServiceImpl>,
    shutdown_signal: Arc<RwLock<Option<tokio::sync::oneshot::Receiver<()>>>>,
}

impl GrpcServer {
    /// Create a new gRPC server
    pub fn new(config: NetworkConfig, service: DatabaseServiceImpl) -> Self {
        Self {
            config,
            service: Arc::new(service),
            shutdown_signal: Arc::new(RwLock::new(None)),
        }
    }

    /// Start the gRPC server
    pub async fn start(&self) -> Result<()> {
        let addr: SocketAddr = self.config.bind_address.parse()
            .map_err(|e| ShardForgeError::Config { 
                message: format!("Invalid bind address: {}", e) 
            })?;

        println!("Starting gRPC server on {}", addr);

        let service = self.service.clone();
        let server = Server::builder()
            .add_service(crate::proto::database_service_server::DatabaseServiceServer::new(service))
            .serve(addr);

        // Handle shutdown signal if available
        if let Some(shutdown_rx) = self.shutdown_signal.read().await.as_ref() {
            server.with_graceful_shutdown(async {
                shutdown_rx.await.ok();
                println!("Gracefully shutting down gRPC server");
            }).await
            .map_err(|e| ShardForgeError::Network { 
                message: format!("gRPC server error: {}", e) 
            })?;
        } else {
            server.await
                .map_err(|e| ShardForgeError::Network { 
                    message: format!("gRPC server error: {}", e) 
                })?;
        }

        Ok(())
    }

    /// Set shutdown signal
    pub async fn set_shutdown_signal(&self, receiver: tokio::sync::oneshot::Receiver<()>) {
        *self.shutdown_signal.write().await = Some(receiver);
    }
}

/// Implementation of the DatabaseService gRPC service
pub struct DatabaseServiceImpl {
    // TODO: Add storage engine, transaction manager, etc.
}

impl DatabaseServiceImpl {
    pub fn new() -> Self {
        Self {}
    }
}

// TODO: Implement the actual gRPC service methods
// This will be completed in Phase 2 when we have storage and transaction management
#[async_trait]
impl crate::proto::database_service_server::DatabaseService for DatabaseServiceImpl {
    async fn execute_query(
        &self,
        request: Request<crate::proto::QueryRequest>,
    ) -> std::result::Result<Response<crate::proto::QueryResponse>, Status> {
        // Placeholder implementation
        let _req = request.into_inner();
        
        let response = crate::proto::QueryResponse {
            result: Some(crate::proto::query_response::Result::Error(
                crate::proto::Error {
                    code: crate::proto::ErrorCode::ServiceUnavailable as i32,
                    message: "Query execution not yet implemented".to_string(),
                    details: "This feature will be available in Phase 1".to_string(),
                    stack_trace: Vec::new(),
                }
            )),
            has_more: false,
            cursor: "".to_string(),
        };

        Ok(Response::new(response))
    }

    async fn begin_transaction(
        &self,
        request: Request<crate::proto::TransactionRequest>,
    ) -> std::result::Result<Response<crate::proto::TransactionResponse>, Status> {
        // Placeholder implementation
        let _req = request.into_inner();
        
        let response = crate::proto::TransactionResponse {
            transaction_id: "tx_placeholder".to_string(),
            start_timestamp: 0,
        };

        Ok(Response::new(response))
    }

    async fn get_cluster_status(
        &self,
        request: Request<crate::proto::StatusRequest>,
    ) -> std::result::Result<Response<crate::proto::StatusResponse>, Status> {
        let _req = request.into_inner();
        
        let response = crate::proto::StatusResponse {
            status: Some(crate::proto::ClusterStatus {
                health: crate::proto::HealthStatus::Healthy as i32,
                total_nodes: 1,
                healthy_nodes: 1,
                leader_node_id: "node-1".to_string(),
            }),
            node_statuses: vec![],
            metrics: None,
        };

        Ok(Response::new(response))
    }

    // TODO: Implement remaining service methods
    // For now, we'll provide placeholder implementations that return "not implemented" errors
    
    async fn join_cluster(
        &self,
        _request: Request<crate::proto::JoinClusterRequest>,
    ) -> std::result::Result<Response<tonic::Streaming<crate::proto::JoinClusterResponse>>, Status> {
        Err(Status::unimplemented("join_cluster not yet implemented"))
    }

    async fn leave_cluster(
        &self,
        _request: Request<crate::proto::LeaveClusterRequest>,
    ) -> std::result::Result<Response<crate::proto::LeaveClusterResponse>, Status> {
        Err(Status::unimplemented("leave_cluster not yet implemented"))
    }

    async fn heartbeat(
        &self,
        _request: Request<crate::proto::HeartbeatRequest>,
    ) -> std::result::Result<Response<crate::proto::HeartbeatResponse>, Status> {
        Err(Status::unimplemented("heartbeat not yet implemented"))
    }

    async fn get_cluster_topology(
        &self,
        _request: Request<crate::proto::TopologyRequest>,
    ) -> std::result::Result<Response<crate::proto::TopologyResponse>, Status> {
        Err(Status::unimplemented("get_cluster_topology not yet implemented"))
    }

    async fn execute_batch(
        &self,
        _request: Request<crate::proto::BatchRequest>,
    ) -> std::result::Result<Response<crate::proto::BatchResponse>, Status> {
        Err(Status::unimplemented("execute_batch not yet implemented"))
    }

    async fn stream_query(
        &self,
        _request: Request<crate::proto::StreamQueryRequest>,
    ) -> std::result::Result<Response<tonic::Streaming<crate::proto::StreamQueryResponse>>, Status> {
        Err(Status::unimplemented("stream_query not yet implemented"))
    }

    async fn prepare_transaction(
        &self,
        _request: Request<crate::proto::PrepareRequest>,
    ) -> std::result::Result<Response<crate::proto::PrepareResponse>, Status> {
        Err(Status::unimplemented("prepare_transaction not yet implemented"))
    }

    async fn commit_transaction(
        &self,
        _request: Request<crate::proto::CommitRequest>,
    ) -> std::result::Result<Response<crate::proto::CommitResponse>, Status> {
        Err(Status::unimplemented("commit_transaction not yet implemented"))
    }

    async fn rollback_transaction(
        &self,
        _request: Request<crate::proto::RollbackRequest>,
    ) -> std::result::Result<Response<crate::proto::RollbackResponse>, Status> {
        Err(Status::unimplemented("rollback_transaction not yet implemented"))
    }

    async fn get_metrics(
        &self,
        _request: Request<crate::proto::MetricsRequest>,
    ) -> std::result::Result<Response<tonic::Streaming<crate::proto::MetricsResponse>>, Status> {
        Err(Status::unimplemented("get_metrics not yet implemented"))
    }

    async fn execute_admin_command(
        &self,
        _request: Request<crate::proto::AdminCommandRequest>,
    ) -> std::result::Result<Response<crate::proto::AdminCommandResponse>, Status> {
        Err(Status::unimplemented("execute_admin_command not yet implemented"))
    }

    async fn get_schema(
        &self,
        _request: Request<crate::proto::SchemaRequest>,
    ) -> std::result::Result<Response<crate::proto::SchemaResponse>, Status> {
        Err(Status::unimplemented("get_schema not yet implemented"))
    }

    async fn update_schema(
        &self,
        _request: Request<crate::proto::UpdateSchemaRequest>,
    ) -> std::result::Result<Response<crate::proto::UpdateSchemaResponse>, Status> {
        Err(Status::unimplemented("update_schema not yet implemented"))
    }

    async fn get_shard_map(
        &self,
        _request: Request<crate::proto::ShardMapRequest>,
    ) -> std::result::Result<Response<crate::proto::ShardMapResponse>, Status> {
        Err(Status::unimplemented("get_shard_map not yet implemented"))
    }
}
