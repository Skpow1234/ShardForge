# System Architecture

## High-Level Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│                       ShardForge Cluster                       │
├─────────────────┬─────────────────┬─────────────────────────────┤
│   CLI Client    │  Query Router   │    Management Console      │
│                 │                 │                             │
│ ┌─────────────┐ │ ┌─────────────┐ │ ┌─────────────────────────┐ │
│ │ SQL Shell   │ │ │ Load        │ │ │ Cluster Management      │ │
│ │ Backup Tool │ │ │ Balancer    │ │ │ Node Health Monitor     │ │
│ │ Admin Utils │ │ │ Connection  │ │ │ Shard Rebalancer        │ │
│ └─────────────┘ │ │ Pool        │ │ └─────────────────────────┘ │
└─────────────────┤ └─────────────┘ │                             │
                  └─────────────────┼─────────────────────────────┤
                                    │        Core Cluster         │
┌─────────────────────────────────────────────────────────────────┤
│                     Shard Groups                               │
│ ┌─────────────────┐ ┌─────────────────┐ ┌─────────────────────┐ │
│ │   Shard 1       │ │   Shard 2       │ │   Shard N           │ │
│ │                 │ │                 │ │                     │ │
│ │ ┌─────┐ ┌─────┐ │ │ ┌─────┐ ┌─────┐ │ │ ┌─────┐ ┌─────┐     │ │
│ │ │Node │ │Node │ │ │ │Node │ │Node │ │ │ │Node │ │Node │     │ │
│ │ │ 1   │ │ 2   │ │ │ │ 3   │ │ 4   │ │ │ │ N-1 │ │ N   │     │ │
│ │ │Lead.│ │Foll.│ │ │ │Lead.│ │Foll.│ │ │ │Lead.│ │Foll.│     │ │
│ │ └─────┘ └─────┘ │ │ └─────┘ └─────┘ │ │ └─────┘ └─────┘     │ │
│ └─────────────────┘ └─────────────────┘ └─────────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

## Component Breakdown

### 1. Node Architecture

Each database node contains:

```rust
pub struct DatabaseNode {
    // Core identification
    pub node_id: NodeId,
    pub cluster_config: ClusterConfig,

    // RAFT consensus
    pub raft_node: RaftNode,

    // Storage engine
    pub storage_engine: StorageEngine,

    // Query processor
    pub query_engine: QueryEngine,

    // Transaction manager
    pub transaction_manager: TransactionManager,

    // Network layer
    pub network_service: NetworkService,

    // Monitoring
    pub metrics_collector: MetricsCollector,
}
```

### 2. Storage Layer Architecture

```text
┌─────────────────────────────────────────────────────────┐
│                  Storage Engine                         │
├─────────────────────────────────────────────────────────┤
│  Write-Ahead Log (WAL)    │    Transaction Log          │
├─────────────────────────────────────────────────────────┤
│              LSM Tree Storage (RocksDB)                 │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   MemTable  │  │  SST Files  │  │ Bloom Filter│      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
├─────────────────────────────────────────────────────────┤
│                  Data Structures                        │
│  ┌─────────────┐  ┌─────────────┐  ┌─────────────┐      │
│  │   B+ Trees  │  │    Indexes  │  │ Statistics  │      │
│  └─────────────┘  └─────────────┘  └─────────────┘      │
└─────────────────────────────────────────────────────────┘
```
