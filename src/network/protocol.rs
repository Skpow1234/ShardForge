//! Protocol buffer definitions and utilities

// Generated protobuf code will be included here when protoc is available
#[cfg(feature = "grpc-enabled")]
pub mod proto {
    tonic::include_proto!("shardforge.rpc");
}

// Stub implementation when protoc is not available
#[cfg(not(feature = "grpc-enabled"))]
pub mod proto {
    use tonic::{Request, Response, Status};

    // Basic stub types for compilation
    #[derive(Debug, Clone)]
    pub struct QueryRequest {
        pub query_id: String,
        pub sql: String,
        pub parameters: Vec<Parameter>,
        pub options: Option<QueryOptions>,
        pub streaming_response: bool,
    }

    #[derive(Debug, Clone)]
    pub struct Parameter {
        pub name: String,
        pub value: Option<Value>,
    }

    #[derive(Debug, Clone)]
    pub struct QueryOptions {
        pub timeout_seconds: u32,
        pub max_rows: Option<u64>,
    }

    #[derive(Debug, Clone)]
    pub struct Value {
        pub value: Option<ValueType>,
    }

    #[derive(Debug, Clone)]
    pub enum ValueType {
        StringVal(String),
        BytesVal(Vec<u8>),
        BoolVal(bool),
        Int32Val(i32),
        Int64Val(i64),
        FloatVal(f32),
        DoubleVal(f64),
    }

    pub mod value {
        pub use super::ValueType as Value;
    }

    #[derive(Debug, Clone)]
    pub struct QueryResponse {
        pub has_more: bool,
        pub cursor: String,
    }

    #[derive(Debug, Clone)]
    pub struct TransactionRequest {
        pub timeout_seconds: u32,
        pub read_only: bool,
        pub isolation_level: i32,
    }

    #[derive(Debug, Clone)]
    pub struct TransactionResponse {
        pub transaction_id: String,
        pub start_timestamp: u64,
    }

    #[derive(Debug, Clone)]
    pub struct CommitRequest {
        pub transaction_id: String,
    }

    #[derive(Debug, Clone)]
    pub struct CommitResponse {
        pub success: bool,
        pub commit_timestamp: u64,
    }

    #[derive(Debug, Clone)]
    pub struct RollbackRequest {
        pub transaction_id: String,
    }

    #[derive(Debug, Clone)]
    pub struct RollbackResponse {
        pub success: bool,
    }

    #[derive(Debug, Clone)]
    pub struct StatusRequest {
        pub include_metrics: bool,
    }

    #[derive(Debug, Clone)]
    pub struct StatusResponse {
        pub status: Option<ClusterStatus>,
    }

    #[derive(Debug, Clone)]
    pub struct ClusterStatus {
        pub health: i32,
        pub total_nodes: u32,
        pub healthy_nodes: u32,
        pub leader_node_id: String,
    }

    #[derive(Debug, Clone)]
    pub struct HeartbeatRequest {
        pub node_id: String,
        pub status: NodeStatus,
    }

    #[derive(Debug, Clone)]
    pub struct HeartbeatResponse {
        pub success: bool,
        pub leader_id: String,
    }

    #[derive(Debug, Clone)]
    pub struct NodeStatus {
        pub health: i32,
        pub load: f32,
    }

    #[derive(Debug, Clone)]
    pub struct TopologyRequest {
        pub include_stats: bool,
    }

    #[derive(Debug, Clone)]
    pub struct TopologyResponse {
        pub nodes: Vec<NodeInfo>,
    }

    #[derive(Debug, Clone)]
    pub struct NodeInfo {
        pub node_id: String,
        pub address: String,
        pub role: i32,
    }

    #[derive(Debug, Clone, Copy)]
    pub enum IsolationLevel {
        ReadUncommitted = 0,
        ReadCommitted = 1,
        RepeatableRead = 2,
        Serializable = 3,
    }

    // Stub service trait
    #[tonic::async_trait]
    pub trait DatabaseService {
        async fn execute_query(
            &self,
            request: Request<QueryRequest>,
        ) -> Result<Response<QueryResponse>, Status>;

        async fn begin_transaction(
            &self,
            request: Request<TransactionRequest>,
        ) -> Result<Response<TransactionResponse>, Status>;

        async fn get_cluster_status(
            &self,
            request: Request<StatusRequest>,
        ) -> Result<Response<StatusResponse>, Status>;
    }

    pub mod database_service_server {
        use super::*;

        pub struct DatabaseServiceServer<T> {
            inner: T,
        }

        impl<T> DatabaseServiceServer<T> {
            pub fn new(inner: T) -> Self {
                Self { inner }
            }
        }
    }

    pub mod database_service_client {
        use super::*;
        use tonic::transport::Channel;

        pub struct DatabaseServiceClient<T> {
            inner: T,
        }

        impl DatabaseServiceClient<Channel> {
            pub fn new(channel: Channel) -> Self {
                Self { inner: channel }
            }

            pub async fn execute_query(
                &mut self,
                request: QueryRequest,
            ) -> Result<Response<QueryResponse>, Status> {
                // Stub implementation
                Ok(Response::new(QueryResponse { has_more: false, cursor: "".to_string() }))
            }

            pub async fn begin_transaction(
                &mut self,
                request: TransactionRequest,
            ) -> Result<Response<TransactionResponse>, Status> {
                Ok(Response::new(TransactionResponse {
                    transaction_id: "tx_stub".to_string(),
                    start_timestamp: 0,
                }))
            }

            pub async fn commit_transaction(
                &mut self,
                request: CommitRequest,
            ) -> Result<Response<CommitResponse>, Status> {
                Ok(Response::new(CommitResponse { success: true, commit_timestamp: 0 }))
            }

            pub async fn rollback_transaction(
                &mut self,
                request: RollbackRequest,
            ) -> Result<Response<RollbackResponse>, Status> {
                Ok(Response::new(RollbackResponse { success: true }))
            }

            pub async fn get_cluster_status(
                &mut self,
                request: StatusRequest,
            ) -> Result<Response<StatusResponse>, Status> {
                Ok(Response::new(StatusResponse {
                    status: Some(ClusterStatus {
                        health: 0,
                        total_nodes: 1,
                        healthy_nodes: 1,
                        leader_node_id: "node-1".to_string(),
                    }),
                }))
            }

            pub async fn heartbeat(
                &mut self,
                request: HeartbeatRequest,
            ) -> Result<Response<HeartbeatResponse>, Status> {
                Ok(Response::new(HeartbeatResponse {
                    success: true,
                    leader_id: "node-1".to_string(),
                }))
            }

            pub async fn get_cluster_topology(
                &mut self,
                request: TopologyRequest,
            ) -> Result<Response<TopologyResponse>, Status> {
                Ok(Response::new(TopologyResponse { nodes: vec![] }))
            }
        }
    }
}

pub use proto::*;

/// Convert from internal types to protobuf types
pub mod conversion {
    use super::proto;
    use shardforge_core::{Key, Value};

    /// Convert internal Value to protobuf Value
    pub fn value_to_proto(value: &Value) -> proto::Value {
        proto::Value {
            value: match value.as_ref() {
                b if b.is_empty() => Some(proto::ValueType::StringVal("".to_string())),
                b => Some(proto::ValueType::BytesVal(b.to_vec())),
            },
        }
    }

    /// Convert protobuf Value to internal Value
    pub fn value_from_proto(proto_value: &proto::Value) -> Value {
        match &proto_value.value {
            Some(proto::ValueType::StringVal(s)) => Value::new(s.as_bytes()),
            Some(proto::ValueType::BytesVal(b)) => Value::new(b),
            Some(proto::ValueType::BoolVal(b)) => Value::new(&[if *b { 1 } else { 0 }]),
            Some(proto::ValueType::Int32Val(i)) => Value::new(&i.to_le_bytes()),
            Some(proto::ValueType::Int64Val(i)) => Value::new(&i.to_le_bytes()),
            Some(proto::ValueType::FloatVal(f)) => Value::new(&f.to_le_bytes()),
            Some(proto::ValueType::DoubleVal(d)) => Value::new(&d.to_le_bytes()),
            _ => Value::new(b""),
        }
    }

    /// Convert internal Key to bytes
    pub fn key_to_bytes(key: &Key) -> Vec<u8> {
        key.as_ref().to_vec()
    }

    /// Convert bytes to internal Key
    pub fn key_from_bytes(bytes: &[u8]) -> Key {
        Key::new(bytes)
    }
}
