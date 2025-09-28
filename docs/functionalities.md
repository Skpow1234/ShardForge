# Core Functionalities

## 1. RAFT Consensus Implementation

### Features Required

- **Leader Election**: Automatic leader selection with configurable timeouts
- **Log Replication**: Reliable log entry replication across followers
- **Membership Changes**: Dynamic cluster reconfiguration
- **Snapshot Support**: Periodic state snapshots for recovery
- **Network Partitions**: Graceful handling of split-brain scenarios

### Implementation Structure

```rust
pub struct RaftNode {
    // Core RAFT state
    current_term: u64,
    voted_for: Option<NodeId>,
    log: Vec<LogEntry>,

    // Volatile state
    commit_index: u64,
    last_applied: u64,

    // Leader state
    next_index: HashMap<NodeId, u64>,
    match_index: HashMap<NodeId, u64>,

    // Configuration
    cluster_members: Vec<NodeId>,
    election_timeout: Duration,
    heartbeat_interval: Duration,
}
```

## 2. Sharding Strategy

### Hash-Based Sharding

```rust
pub enum ShardingStrategy {
    Hash(HashSharding),
    Range(RangeSharding),
    Directory(DirectorySharding),
}

pub struct HashSharding {
    shard_count: u32,
    hash_function: HashFunction,
    virtual_nodes: u32, // For consistent hashing
}
```

### Shard Management Features

- **Automatic Rebalancing**: Monitor load and redistribute shards
- **Split/Merge Operations**: Handle shard size optimization
- **Cross-Shard Transactions**: Distributed transaction coordination
- **Shard Recovery**: Rebuild shards from replicas

## 3. ACID Transaction Support

### Transaction Manager

```rust
pub struct TransactionManager {
    active_transactions: HashMap<TransactionId, Transaction>,
    lock_manager: LockManager,
    deadlock_detector: DeadlockDetector,
    mvcc_manager: MVCCManager,
}

pub struct Transaction {
    id: TransactionId,
    timestamp: Timestamp,
    isolation_level: IsolationLevel,
    read_set: HashSet<RowId>,
    write_set: HashMap<RowId, Value>,
    state: TransactionState,
}
```

### ACID Properties Implementation

- **Atomicity**: Two-phase commit protocol across shards
- **Consistency**: Constraint validation and trigger execution
- **Isolation**: Multi-Version Concurrency Control (MVCC)
- **Durability**: Write-ahead logging with fsync guarantees

## 4. SQL Compatibility Layer

### Supported SQL Features (Phase 1)

```sql
-- Data Definition Language (DDL)
CREATE TABLE, ALTER TABLE, DROP TABLE
CREATE INDEX, DROP INDEX
CREATE SCHEMA, DROP SCHEMA

-- Data Manipulation Language (DML)
SELECT (with JOINs, subqueries, aggregates)
INSERT, UPDATE, DELETE
UPSERT (INSERT ... ON CONFLICT)

-- Data Control Language (DCL)
GRANT, REVOKE (basic permissions)

-- Transaction Control
BEGIN, COMMIT, ROLLBACK
SAVEPOINT, ROLLBACK TO SAVEPOINT
```

### Query Engine Pipeline

```rust
pub struct QueryEngine {
    parser: SQLParser,
    planner: QueryPlanner,
    optimizer: QueryOptimizer,
    executor: QueryExecutor,
}

// Query processing flow:
// SQL -> AST -> Logical Plan -> Physical Plan -> Execution
```

## 5. CLI Interface (Year 1 Focus)

### Core CLI Commands

```bash
# Cluster Management
shardforge cluster init --nodes 3 --replication-factor 3
shardforge cluster add-node --host 10.0.0.4 --port 5432
shardforge cluster remove-node --node-id node-4
shardforge cluster status

# Database Operations
shardforge database create mydb
shardforge database list
shardforge database drop mydb

# Interactive SQL Shell
shardforge sql --database mydb
shardforge sql --file queries.sql --database mydb

# Backup & Recovery
shardforge backup create --database mydb --output backup.tar.gz
shardforge backup restore --input backup.tar.gz --database mydb_restored

# Monitoring & Diagnostics
shardforge metrics --format json
shardforge logs --level debug --follow
shardforge performance-report --database mydb
```

### CLI Architecture

```rust
pub struct CliApp {
    config: CliConfig,
    client: DatabaseClient,
    command_processor: CommandProcessor,
}

pub enum CliCommand {
    Cluster(ClusterCommand),
    Database(DatabaseCommand),
    Sql(SqlCommand),
    Backup(BackupCommand),
    Metrics(MetricsCommand),
}
```
