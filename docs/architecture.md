# System Architecture

## High-Level Architecture

ShardForge follows a layered architecture designed for high availability, horizontal scalability, and strong consistency. The system is built around the RAFT consensus protocol with intelligent sharding for distributed data management.

```text
┌─────────────────────────────────────────────────────────────────────────────────┐
│                             ShardForge Cluster                                  │
├─────────────────┬─────────────────┬─────────────────┬───────────────────────────┤
│   CLI Tools     │  Query Router   │  Load Balancer  │   Management Console      │
│                 │                 │                 │                           │
│ ┌─────────────┐ │ ┌─────────────┐ │ ┌─────────────┐ │ ┌───────────────────────┐ │
│ │ SQL Shell   │ │ │ Query       │ │ │ Connection  │ │ │ Cluster Management    │ │
│ │ Admin Utils │ │ │ Coordinator │ │ │ Pool        │ │ │ Health Monitoring     │ │
│ │ Backup/Restore│ │ │ Shard      │ │ │ Failover    │ │ │ Configuration Mgmt    │ │
│ └─────────────┘ │ │ Router      │ │ └─────────────┘ │ └───────────────────────┘ │
└─────────────────┤ └─────────────┘ └─────────────────┼───────────────────────────┤
                  └──────────────────────────────────┼───────────────────────────┤
                                                     │     Distributed Core       │
┌─────────────────────────────────────────────────────────────────────────────────┤
│                              Shard Groups                                     │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ │
│ │   Shard 1       │ │   Shard 2       │ │   Shard 3       │ │   Shard N       │ │
│ │ (Hash Range     │ │ (Hash Range     │ │ (Hash Range     │ │ (Hash Range     │ │
│ │  0x0000-0x3FFF) │ │  0x4000-0x7FFF) │ │  0x8000-0xBFFF) │ │  0xC000-0xFFFF) │ │
│ │                 │ │                 │ │                 │ │                 │ │
│ │ ┌─────┐ ┌─────┐ │ │ ┌─────┐ ┌─────┐ │ │ ┌─────┐ ┌─────┐ │ │ ┌─────┐ ┌─────┐ │ │
│ │ │Node │ │Node │ │ │ │Node │ │Node │ │ │ │Node │ │Node │ │ │ │Node │ │Node │ │ │
│ │ │ 1   │ │ 2   │ │ │ │ 3   │ │ 4   │ │ │ │ 5   │ │ 6   │ │ │ │N-1  │ │ N   │ │ │
│ │ │Lead.│ │Repl.│ │ │ │Lead.│ │Repl.│ │ │ │Lead.│ │Repl.│ │ │ │Lead.│ │Repl.│ │ │
│ │ └─────┘ └─────┘ │ │ └─────┘ └─────┘ │ │ └─────┘ └─────┘ │ │ └─────┘ └─────┘ │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────┘ └─────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────────┘
                                                     │
                                                     ▼
┌─────────────────────────────────────────────────────────────────────────────────┐
│                              Metadata Service                                  │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────┐ │
│ │ Schema Registry │ │ Cluster Topology│ │ Shard Map       │ │ Transaction     │ │
│ │ DDL Operations │ │ Membership      │ │ Routing Rules   │ │ Coordinator     │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────┘ └─────────────────┘ │
└─────────────────────────────────────────────────────────────────────────────────┘
```

## Architecture Principles

### 1. Separation of Concerns

- **Data Layer**: Storage engines, indexing, and data persistence
- **Compute Layer**: Query processing, transaction management, and execution
- **Coordination Layer**: Consensus, cluster management, and metadata
- **Access Layer**: Client interfaces, load balancing, and API gateways

### 2. Failure Domains

- **Node Isolation**: Individual node failures don't cascade
- **Shard Isolation**: Shard failures contained within replica groups
- **Network Partition Tolerance**: Graceful degradation under split-brain scenarios

## Component Breakdown

### 1. Node Architecture

Each database node follows a modular architecture with clear separation of concerns and dependency injection patterns:

```rust
pub struct DatabaseNode {
    // Core identification and lifecycle
    pub node_id: NodeId,
    pub cluster_config: Arc<ClusterConfig>,
    pub node_state: Arc<RwLock<NodeState>>,

    // Core services (injected via dependency injection)
    pub raft_service: Arc<RaftService>,
    pub storage_service: Arc<StorageService>,
    pub query_service: Arc<QueryService>,
    pub transaction_service: Arc<TransactionService>,
    pub network_service: Arc<NetworkService>,

    // Cross-cutting concerns
    pub metrics_registry: Arc<MetricsRegistry>,
    pub health_checker: Arc<HealthChecker>,
    pub circuit_breaker: Arc<CircuitBreaker>,

    // Async runtime and task management
    pub runtime_handle: tokio::runtime::Handle,
    pub task_manager: Arc<TaskManager>,
}

impl DatabaseNode {
    pub async fn new(config: ClusterConfig) -> Result<Self, NodeError> {
        // Initialize components with proper error handling and cleanup
        let metrics = Arc::new(MetricsRegistry::new());
        let health_checker = Arc::new(HealthChecker::new(metrics.clone()));

        // Initialize storage first (most critical)
        let storage = Arc::new(StorageService::init(&config.storage).await?);

        // Initialize network layer
        let network = Arc::new(NetworkService::new(&config.network, health_checker.clone()).await?);

        // Initialize RAFT consensus
        let raft = Arc::new(RaftService::new(&config.raft, storage.clone(), network.clone()).await?);

        // Initialize transaction and query services
        let transaction_svc = Arc::new(TransactionService::new(storage.clone(), raft.clone()).await?);
        let query_svc = Arc::new(QueryService::new(storage.clone(), transaction_svc.clone()).await?);

        Ok(Self {
            node_id: config.node_id,
            cluster_config: Arc::new(config),
            raft_service: raft,
            storage_service: storage,
            query_service: query_svc,
            transaction_service: transaction_svc,
            network_service: network,
            metrics_registry: metrics,
            health_checker,
            circuit_breaker: Arc::new(CircuitBreaker::new()),
            runtime_handle: tokio::runtime::Handle::current(),
            task_manager: Arc::new(TaskManager::new()),
            node_state: Arc::new(RwLock::new(NodeState::Initializing)),
        })
    }

    pub async fn start(&self) -> Result<(), NodeError> {
        // Start services in dependency order
        self.storage_service.start().await?;
        self.network_service.start().await?;
        self.raft_service.start().await?;
        self.transaction_service.start().await?;
        self.query_service.start().await?;

        *self.node_state.write().await = NodeState::Running;
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<(), NodeError> {
        // Graceful shutdown in reverse dependency order
        self.query_service.stop().await?;
        self.transaction_service.stop().await?;
        self.raft_service.stop().await?;
        self.network_service.stop().await?;
        self.storage_service.stop().await?;

        *self.node_state.write().await = NodeState::Shutdown;
        Ok(())
    }
}
```

### 2. Storage Layer Architecture

The storage layer implements a multi-tiered architecture with pluggable storage engines, comprehensive caching, and advanced indexing capabilities:

```text
┌─────────────────────────────────────────────────────────────────────────────────┐
│                             Storage Service Layer                              │
├─────────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │  Query Cache    │  │  Buffer Pool    │  │  WAL Manager    │  │  Compaction │ │
│  │  (Hot Data)     │  │  (Working Set)  │  │  (Durability)   │  │  Scheduler  │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────────────────────────────┤
│                            Storage Engine Abstraction                          │
├─────────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │   RocksDB       │  │   Sled          │  │   TiKV          │  │   Custom    │ │
│  │   (Primary)     │  │   (Alternative) │  │   (Compatible)  │  │   Engine    │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────────────────────────────┤
│                              Data Organization                                 │
├─────────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │  LSM Trees      │  │  B+ Trees       │  │  Hash Indexes   │  │  Full-text  │ │
│  │  (Primary)      │  │  (Secondary)    │  │  (Fast Lookup)  │  │  Search     │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  └─────────────┘ │
├─────────────────────────────────────────────────────────────────────────────────┤
│                              Physical Storage                                  │
├─────────────────────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────┐ │
│  │  SSD Optimized  │  │  NVMe Support   │  │  RAID Arrays    │  │  Cloud       │ │
│  │  Storage        │  │  (High Perf)    │  │  (Reliability)  │  │  Storage     │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘  └─────────────┘ │
└─────────────────────────────────────────────────────────────────────────────────┘
```

#### Storage Engine Interface

```rust
#[async_trait]
pub trait StorageEngine: Send + Sync {
    /// Core storage operations
    async fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>, StorageError>;
    async fn put(&self, key: &[u8], value: &[u8]) -> Result<(), StorageError>;
    async fn delete(&self, key: &[u8]) -> Result<(), StorageError>;

    /// Batch operations for efficiency
    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<(), StorageError>;

    /// Range queries and iteration
    async fn range_scan(&self, start: &[u8], end: &[u8]) -> Result<StorageIterator, StorageError>;

    /// Transaction support
    async fn begin_transaction(&self) -> Result<TransactionHandle, StorageError>;
    async fn commit_transaction(&self, tx: TransactionHandle) -> Result<(), StorageError>;
    async fn rollback_transaction(&self, tx: TransactionHandle) -> Result<(), StorageError>;

    /// Administrative operations
    async fn flush(&self) -> Result<(), StorageError>;
    async fn compact(&self, range: Option<KeyRange>) -> Result<(), StorageError>;
    fn stats(&self) -> StorageStats;
}

/// Pluggable storage engines
pub enum StorageEngineType {
    RocksDB(RocksDBEngine),
    Sled(SledEngine),
    InMemory(MemoryEngine), // For testing
    Custom(Box<dyn StorageEngine>),
}
```

#### Advanced Features

- **Multi-Version Concurrency Control (MVCC)**: Versioned storage with snapshot isolation
- **Prefix Compression**: Efficient storage of similar keys
- **Bloom Filters**: Fast existence checks with minimal false positives
- **Compaction Strategies**: Tiered, leveled, and universal compaction algorithms
- **Checksum Validation**: Data integrity verification at all levels
- **Backup Integration**: Point-in-time recovery and incremental backups
