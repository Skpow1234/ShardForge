# System Architecture

## High-Level Architecture

ShardForge follows a layered architecture designed for high availability, horizontal scalability, and strong consistency. The system is built around the RAFT consensus protocol with intelligent sharding for distributed data management.

## Project Folder Structure

Based on industry best practices for database systems, ShardForge organizes its codebase into a clear, modular folder structure that separates concerns while maintaining clean dependencies:

```text
shardforge/
├── src/                          # Main source code
│   ├── core/                     # Core types and utilities
│   │   ├── error.rs             # Error types and handling
│   │   ├── types.rs             # Core data types and identifiers
│   │   ├── node.rs              # Node management and health
│   │   ├── cluster.rs           # Cluster topology and coordination
│   │   └── mod.rs               # Module declarations
│   │
│   ├── storage/                  # Storage engine abstraction layer
│   │   ├── engine.rs            # StorageEngine trait and factory
│   │   ├── iterator.rs          # Iterator interface for range queries
│   │   ├── memory.rs            # In-memory storage engine
│   │   ├── rocksdb.rs           # RocksDB storage engine
│   │   ├── sled.rs              # Sled storage engine
│   │   └── mod.rs               # Module declarations
│   │
│   ├── sql/                     # SQL processing layer
│   │   ├── parser.rs            # SQL parser and AST
│   │   ├── planner.rs           # Query planning and optimization
│   │   ├── executor.rs          # Query execution engine
│   │   ├── catalog.rs           # Schema and metadata management
│   │   └── mod.rs               # Module declarations
│   │
│   ├── transaction/             # Transaction management layer
│   │   ├── manager.rs           # Transaction coordinator
│   │   ├── isolation.rs         # Isolation levels and concurrency
│   │   ├── lock.rs              # Lock management
│   │   ├── mvcc.rs              # Multi-version concurrency control
│   │   └── mod.rs               # Module declarations
│   │
│   ├── consensus/               # Consensus and coordination layer
│   │   ├── raft.rs              # RAFT consensus implementation
│   │   ├── membership.rs        # Cluster membership management
│   │   ├── log.rs               # Log replication and persistence
│   │   ├── election.rs          # Leader election logic
│   │   └── mod.rs               # Module declarations
│   │
│   ├── network/                 # Networking and communication layer
│   │   ├── server.rs            # RPC server implementation
│   │   ├── client.rs            # RPC client for cluster communication
│   │   ├── protocol.rs          # Wire protocol definitions
│   │   ├── connection.rs        # Connection pooling and management
│   │   └── mod.rs               # Module declarations
│   │
│   ├── config/                  # Configuration management layer
│   │   ├── config.rs            # Configuration structures
│   │   ├── loader.rs            # Configuration loading logic
│   │   ├── validator.rs         # Configuration validation
│   │   └── mod.rs               # Module declarations
│   │
│   ├── server/                  # Main server orchestration
│   │   ├── node.rs              # Database node implementation
│   │   ├── coordinator.rs       # Request coordination
│   │   ├── lifecycle.rs         # Server lifecycle management
│   │   └── mod.rs               # Module declarations
│   │
│   └── lib.rs                   # Main library entry point
│
├── bin/                          # Binary executables
│   └── shardforge.rs            # Main CLI binary
│
├── tests/                        # Integration tests
│   ├── storage/                 # Storage engine tests
│   ├── sql/                     # SQL processing tests
│   ├── transaction/             # Transaction tests
│   ├── consensus/               # Consensus tests
│   ├── network/                 # Network tests
│   └── integration.rs           # Full system integration tests
│
├── examples/                     # Example applications
│   ├── basic_usage.rs           # Basic database usage
│   ├── cluster_setup.rs         # Multi-node cluster setup
│   ├── custom_storage.rs        # Custom storage engine
│   └── benchmark.rs             # Performance benchmarking
│
├── benches/                      # Performance benchmarks
│   ├── storage.rs               # Storage engine benchmarks
│   ├── sql.rs                   # SQL processing benchmarks
│   ├── transaction.rs           # Transaction benchmarks
│   └── consensus.rs             # Consensus benchmarks
│
├── docs/                         # Documentation
│   ├── architecture.md          # System architecture
│   ├── deployment.md            # Deployment guide
│   ├── security.md              # Security guide
│   ├── testing.md               # Testing strategy
│   ├── roadmap.md               # Development roadmap
│   └── api/                     # API documentation
│
├── docker/                       # Docker-related files
│   ├── Dockerfile               # Main Dockerfile
│   ├── docker-compose.yml       # Development environment
│   ├── Dockerfile.test          # Test environment
│   └── scripts/                 # Docker build scripts
│
├── scripts/                      # Build and utility scripts
│   ├── build.sh                 # Build script
│   ├── test.sh                  # Test runner
│   ├── benchmark.sh             # Benchmark runner
│   ├── ci/                      # CI/CD scripts
│   └── dev/                     # Development utilities
│
├── config/                       # Configuration templates
│   ├── default.toml             # Default configuration
│   ├── development.toml         # Development settings
│   ├── production.toml          # Production settings
│   └── examples/                # Configuration examples
│
├── tools/                        # Development tools
│   ├── codegen/                 # Code generation tools
│   ├── lint/                    # Linting and formatting
│   ├── perf/                    # Performance analysis
│   └── migration/               # Schema migration tools
│
├── Cargo.toml                    # Workspace configuration
├── Cargo.lock                    # Dependency lock file
├── README.md                     # Project documentation
├── LICENSE                       # License file
├── .gitignore                   # Git ignore rules
├── rustfmt.toml                 # Code formatting configuration
├── clippy.toml                  # Linting configuration
└── .github/                      # GitHub configuration
    ├── workflows/               # CI/CD workflows
    └── ISSUE_TEMPLATE/          # Issue templates
```

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
