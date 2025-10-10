//! Transaction manager implementation

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use shardforge_core::{Result, ShardForgeError, Timestamp, TransactionId};
use shardforge_storage::MVCCStorage;
use tokio::sync::{Mutex, RwLock};

/// Transaction state
#[derive(Debug, Clone, PartialEq)]
pub enum TransactionState {
    /// Transaction is active and can perform operations
    Active,
    /// Transaction is preparing to commit (2PC phase 1)
    Preparing,
    /// Transaction is committing (2PC phase 2)
    Committing,
    /// Transaction has been committed successfully
    Committed,
    /// Transaction is rolling back
    RollingBack,
    /// Transaction has been rolled back
    Aborted,
}

/// Isolation level for transactions
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IsolationLevel {
    /// May read uncommitted changes
    ReadUncommitted,
    /// Only read committed changes
    ReadCommitted,
    /// Consistent reads within transaction
    RepeatableRead,
    /// Full isolation, no concurrency anomalies
    Serializable,
}

/// Transaction configuration
#[derive(Debug, Clone)]
pub struct TransactionConfig {
    /// Isolation level
    pub isolation_level: IsolationLevel,
    /// Transaction timeout
    pub timeout: Duration,
    /// Read-only transaction
    pub read_only: bool,
}

impl Default for TransactionConfig {
    fn default() -> Self {
        Self {
            isolation_level: IsolationLevel::ReadCommitted,
            timeout: Duration::from_secs(30),
            read_only: false,
        }
    }
}

/// Transaction context
#[derive(Debug)]
pub struct Transaction {
    /// Unique transaction identifier
    pub id: TransactionId,
    /// Transaction start timestamp
    pub start_timestamp: Timestamp,
    /// Current state
    pub state: TransactionState,
    /// Configuration
    pub config: TransactionConfig,
    /// Start time for timeout tracking
    pub start_time: Instant,
    /// Read set for conflict detection
    pub read_set: HashMap<String, Timestamp>,
    /// Write set for buffering changes
    pub write_set: HashMap<String, WriteIntent>,
    /// Lock set for acquired locks
    pub lock_set: HashSet<String>,
}

/// Write intent for uncommitted changes
#[derive(Debug, Clone)]
pub struct WriteIntent {
    /// The value to write (None for delete)
    pub value: Option<Vec<u8>>,
    /// Timestamp when write was made
    pub timestamp: Timestamp,
}

/// Lock manager for concurrency control
pub struct LockManager {
    /// Active locks by resource
    locks: Arc<RwLock<HashMap<String, LockInfo>>>,
    /// Wait-for graph for deadlock detection
    wait_for: Arc<RwLock<HashMap<TransactionId, HashSet<TransactionId>>>>,
}

/// Lock information
#[derive(Debug, Clone)]
pub struct LockInfo {
    /// Transaction holding the lock
    pub holder: TransactionId,
    /// Lock type
    pub lock_type: LockType,
    /// Transactions waiting for this lock
    pub waiters: Vec<TransactionId>,
}

/// Lock types
#[derive(Debug, Clone, PartialEq)]
pub enum LockType {
    /// Shared lock for reads
    Shared,
    /// Exclusive lock for writes
    Exclusive,
}

/// Transaction manager
pub struct TransactionManager {
    /// Active transactions
    transactions: Arc<RwLock<HashMap<TransactionId, Arc<Mutex<Transaction>>>>>,
    /// MVCC storage layer
    mvcc_storage: Arc<MVCCStorage>,
    /// Lock manager
    lock_manager: Arc<LockManager>,
    /// Timestamp oracle for generating timestamps
    timestamp_counter: Arc<RwLock<u64>>,
}

impl Transaction {
    /// Create a new transaction
    pub fn new(id: TransactionId, start_timestamp: Timestamp, config: TransactionConfig) -> Self {
        Self {
            id,
            start_timestamp,
            state: TransactionState::Active,
            config,
            start_time: Instant::now(),
            read_set: HashMap::new(),
            write_set: HashMap::new(),
            lock_set: HashSet::new(),
        }
    }

    /// Check if transaction has timed out
    pub fn is_timed_out(&self) -> bool {
        self.start_time.elapsed() > self.config.timeout
    }

    /// Add a read to the read set
    pub fn add_read(&mut self, key: String, timestamp: Timestamp) {
        self.read_set.insert(key, timestamp);
    }

    /// Add a write to the write set
    pub fn add_write(&mut self, key: String, value: Option<Vec<u8>>, timestamp: Timestamp) {
        self.write_set.insert(key, WriteIntent { value, timestamp });
    }

    /// Add a lock to the lock set
    pub fn add_lock(&mut self, resource: String) {
        self.lock_set.insert(resource);
    }

    /// Check if transaction can proceed based on state
    pub fn can_proceed(&self) -> bool {
        matches!(self.state, TransactionState::Active | TransactionState::Preparing)
    }
}

impl LockManager {
    /// Create a new lock manager
    pub fn new() -> Self {
        Self {
            locks: Arc::new(RwLock::new(HashMap::new())),
            wait_for: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Acquire a lock on a resource
    pub async fn acquire_lock(
        &self,
        transaction_id: TransactionId,
        resource: String,
        lock_type: LockType,
    ) -> Result<()> {
        let mut locks = self.locks.write().await;

        if let Some(lock_info) = locks.get_mut(&resource) {
            // Resource is already locked
            if lock_info.holder == transaction_id {
                // We already hold the lock
                return Ok(());
            }

            if lock_type == LockType::Shared && lock_info.lock_type == LockType::Shared {
                // Compatible shared locks - upgrade holder to include this transaction
                return Ok(());
            }

            // Incompatible lock - add to waiters and detect deadlock
            lock_info.waiters.push(transaction_id);

            // Add to wait-for graph
            let mut wait_for = self.wait_for.write().await;
            wait_for
                .entry(transaction_id)
                .or_insert_with(HashSet::new)
                .insert(lock_info.holder);

            // Check for deadlock
            if self.has_deadlock(&wait_for, transaction_id) {
                // Remove from waiters and wait-for graph
                lock_info.waiters.retain(|&id| id != transaction_id);
                wait_for.get_mut(&transaction_id).unwrap().remove(&lock_info.holder);

                return Err(ShardForgeError::Transaction {
                    message: "Deadlock detected".to_string(),
                });
            }

            // Wait for lock (in real implementation, this would be a proper wait/notify mechanism)
            return Err(ShardForgeError::Transaction {
                message: "Resource is locked, would need to wait".to_string(),
            });
        } else {
            // Resource is not locked - acquire it
            locks.insert(
                resource,
                LockInfo {
                    holder: transaction_id,
                    lock_type,
                    waiters: Vec::new(),
                },
            );
        }

        Ok(())
    }

    /// Release a lock on a resource
    pub async fn release_lock(&self, transaction_id: TransactionId, resource: &str) -> Result<()> {
        let mut locks = self.locks.write().await;

        if let Some(lock_info) = locks.get(resource) {
            if lock_info.holder == transaction_id {
                if lock_info.waiters.is_empty() {
                    // No waiters - remove lock
                    locks.remove(resource);
                } else {
                    // Give lock to first waiter
                    let next_holder = lock_info.waiters[0];
                    let mut new_lock_info = lock_info.clone();
                    new_lock_info.holder = next_holder;
                    new_lock_info.waiters.remove(0);
                    locks.insert(resource.to_string(), new_lock_info);

                    // Remove from wait-for graph
                    let mut wait_for = self.wait_for.write().await;
                    if let Some(waiting_for) = wait_for.get_mut(&next_holder) {
                        waiting_for.remove(&transaction_id);
                        if waiting_for.is_empty() {
                            wait_for.remove(&next_holder);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Release all locks held by a transaction
    pub async fn release_all_locks(&self, transaction_id: TransactionId) -> Result<()> {
        let mut locks = self.locks.write().await;
        let mut resources_to_remove = Vec::new();
        let mut resources_to_update = Vec::new();

        for (resource, lock_info) in locks.iter() {
            if lock_info.holder == transaction_id {
                if lock_info.waiters.is_empty() {
                    resources_to_remove.push(resource.clone());
                } else {
                    resources_to_update.push((resource.clone(), lock_info.clone()));
                }
            }
        }

        // Remove locks with no waiters
        for resource in resources_to_remove {
            locks.remove(&resource);
        }

        // Update locks with waiters
        for (resource, mut lock_info) in resources_to_update {
            let next_holder = lock_info.waiters.remove(0);
            lock_info.holder = next_holder;
            locks.insert(resource, lock_info);
        }

        // Clean up wait-for graph
        let mut wait_for = self.wait_for.write().await;
        wait_for.retain(|_, waiting_for| {
            waiting_for.remove(&transaction_id);
            !waiting_for.is_empty()
        });

        Ok(())
    }

    /// Check for deadlocks using cycle detection
    fn has_deadlock(
        &self,
        wait_for: &HashMap<TransactionId, HashSet<TransactionId>>,
        start: TransactionId,
    ) -> bool {
        let mut visited = HashSet::new();
        let mut path = HashSet::new();
        self.dfs_cycle_detection(wait_for, start, &mut visited, &mut path)
    }

    /// Depth-first search for cycle detection
    fn dfs_cycle_detection(
        &self,
        wait_for: &HashMap<TransactionId, HashSet<TransactionId>>,
        current: TransactionId,
        visited: &mut HashSet<TransactionId>,
        path: &mut HashSet<TransactionId>,
    ) -> bool {
        if path.contains(&current) {
            return true; // Cycle found
        }

        if visited.contains(&current) {
            return false; // Already explored
        }

        visited.insert(current);
        path.insert(current);

        if let Some(dependencies) = wait_for.get(&current) {
            for &dependency in dependencies {
                if self.dfs_cycle_detection(wait_for, dependency, visited, path) {
                    return true;
                }
            }
        }

        path.remove(&current);
        false
    }
}

impl TransactionManager {
    /// Create a new transaction manager
    pub fn new(mvcc_storage: Arc<MVCCStorage>) -> Self {
        Self {
            transactions: Arc::new(RwLock::new(HashMap::new())),
            mvcc_storage,
            lock_manager: Arc::new(LockManager::new()),
            timestamp_counter: Arc::new(RwLock::new(1)),
        }
    }

    /// Generate a new timestamp
    pub async fn next_timestamp(&self) -> Timestamp {
        let mut counter = self.timestamp_counter.write().await;
        let timestamp = *counter;
        *counter += 1;
        Timestamp::new(timestamp)
    }

    /// Begin a new transaction
    pub async fn begin_transaction(&self, config: TransactionConfig) -> Result<TransactionId> {
        let transaction_id = TransactionId::new();
        let start_timestamp = self.mvcc_storage.begin_transaction(transaction_id).await;

        let transaction = Transaction::new(transaction_id, start_timestamp, config);

        let mut transactions = self.transactions.write().await;
        transactions.insert(transaction_id, Arc::new(Mutex::new(transaction)));

        Ok(transaction_id)
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, transaction_id: TransactionId) -> Result<()> {
        let transaction_arc = {
            let transactions = self.transactions.read().await;
            transactions.get(&transaction_id).cloned().ok_or_else(|| {
                ShardForgeError::Transaction {
                    message: format!("Transaction {:?} not found", transaction_id),
                }
            })?
        };

        let mut transaction = transaction_arc.lock().await;

        // Check transaction state
        if !transaction.can_proceed() {
            return Err(ShardForgeError::Transaction {
                message: format!(
                    "Transaction {:?} cannot be committed in state {:?}",
                    transaction_id, transaction.state
                ),
            });
        }

        // Check for timeout
        if transaction.is_timed_out() {
            transaction.state = TransactionState::Aborted;
            return Err(ShardForgeError::Transaction {
                message: "Transaction timed out".to_string(),
            });
        }

        // Change state to committing
        transaction.state = TransactionState::Committing;

        // Apply writes to MVCC storage
        for (key, write_intent) in &transaction.write_set {
            self.mvcc_storage
                .write(
                    shardforge_core::Key::new(key.as_bytes()),
                    write_intent.value.as_ref().map(|v| shardforge_core::Value::new(v)),
                    transaction_id,
                    write_intent.timestamp,
                )
                .await?;
        }

        // Commit in MVCC storage
        self.mvcc_storage.commit_transaction(transaction_id).await?;

        // Release all locks
        self.lock_manager.release_all_locks(transaction_id).await?;

        // Update transaction state
        transaction.state = TransactionState::Committed;

        // Remove from active transactions
        drop(transaction);
        let mut transactions = self.transactions.write().await;
        transactions.remove(&transaction_id);

        Ok(())
    }

    /// Rollback a transaction
    pub async fn rollback_transaction(&self, transaction_id: TransactionId) -> Result<()> {
        let transaction_arc = {
            let transactions = self.transactions.read().await;
            transactions.get(&transaction_id).cloned().ok_or_else(|| {
                ShardForgeError::Transaction {
                    message: format!("Transaction {:?} not found", transaction_id),
                }
            })?
        };

        let mut transaction = transaction_arc.lock().await;

        // Change state to rolling back
        transaction.state = TransactionState::RollingBack;

        // Abort in MVCC storage
        self.mvcc_storage.abort_transaction(transaction_id).await?;

        // Release all locks
        self.lock_manager.release_all_locks(transaction_id).await?;

        // Update transaction state
        transaction.state = TransactionState::Aborted;

        // Remove from active transactions
        drop(transaction);
        let mut transactions = self.transactions.write().await;
        transactions.remove(&transaction_id);

        Ok(())
    }

    /// Get transaction state
    pub async fn get_transaction_state(
        &self,
        transaction_id: TransactionId,
    ) -> Option<TransactionState> {
        let transactions = self.transactions.read().await;
        if let Some(transaction_arc) = transactions.get(&transaction_id) {
            let transaction = transaction_arc.lock().await;
            Some(transaction.state.clone())
        } else {
            None
        }
    }

    /// Check if a transaction is active
    pub async fn is_transaction_active(&self, transaction_id: TransactionId) -> bool {
        matches!(self.get_transaction_state(transaction_id).await, Some(TransactionState::Active))
    }

    /// Get number of active transactions
    pub async fn active_transaction_count(&self) -> usize {
        let transactions = self.transactions.read().await;
        transactions.len()
    }

    /// Cleanup timed out transactions
    pub async fn cleanup_timed_out_transactions(&self) -> Result<Vec<TransactionId>> {
        let mut timed_out = Vec::new();
        let transactions = self.transactions.read().await;

        for (&transaction_id, transaction_arc) in transactions.iter() {
            let transaction = transaction_arc.lock().await;
            if transaction.is_timed_out() && transaction.can_proceed() {
                timed_out.push(transaction_id);
            }
        }

        drop(transactions);

        // Rollback timed out transactions
        for transaction_id in &timed_out {
            if let Err(e) = self.rollback_transaction(*transaction_id).await {
                tracing::error!(
                    "Failed to rollback timed out transaction {:?}: {}",
                    transaction_id,
                    e
                );
            }
        }

        Ok(timed_out)
    }
}

impl Default for LockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shardforge_storage::MVCCStorage;

    #[tokio::test]
    async fn test_transaction_lifecycle() {
        let mvcc_storage = Arc::new(MVCCStorage::new());
        let tx_manager = TransactionManager::new(mvcc_storage);

        // Begin transaction
        let tx_id = tx_manager.begin_transaction(TransactionConfig::default()).await.unwrap();
        assert!(tx_manager.is_transaction_active(tx_id).await);

        // Commit transaction
        tx_manager.commit_transaction(tx_id).await.unwrap();
        assert!(!tx_manager.is_transaction_active(tx_id).await);
    }

    #[tokio::test]
    async fn test_transaction_rollback() {
        let mvcc_storage = Arc::new(MVCCStorage::new());
        let tx_manager = TransactionManager::new(mvcc_storage);

        // Begin transaction
        let tx_id = tx_manager.begin_transaction(TransactionConfig::default()).await.unwrap();
        assert!(tx_manager.is_transaction_active(tx_id).await);

        // Rollback transaction
        tx_manager.rollback_transaction(tx_id).await.unwrap();
        assert!(!tx_manager.is_transaction_active(tx_id).await);
    }

    #[tokio::test]
    async fn test_lock_acquisition() {
        let lock_manager = LockManager::new();
        let tx1 = TransactionId::new();
        let tx2 = TransactionId::new();

        // tx1 acquires exclusive lock
        lock_manager
            .acquire_lock(tx1, "resource1".to_string(), LockType::Exclusive)
            .await
            .unwrap();

        // tx2 tries to acquire exclusive lock on same resource - should fail
        let result = lock_manager
            .acquire_lock(tx2, "resource1".to_string(), LockType::Exclusive)
            .await;
        assert!(result.is_err());

        // Release lock
        lock_manager.release_lock(tx1, "resource1").await.unwrap();
    }

    #[tokio::test]
    async fn test_shared_locks() {
        let lock_manager = LockManager::new();
        let tx1 = TransactionId::new();
        let tx2 = TransactionId::new();

        // tx1 acquires shared lock
        lock_manager
            .acquire_lock(tx1, "resource1".to_string(), LockType::Shared)
            .await
            .unwrap();

        // tx2 acquires shared lock on same resource - should succeed
        lock_manager
            .acquire_lock(tx2, "resource1".to_string(), LockType::Shared)
            .await
            .unwrap();

        // Release locks
        lock_manager.release_lock(tx1, "resource1").await.unwrap();
        lock_manager.release_lock(tx2, "resource1").await.unwrap();
    }

    #[tokio::test]
    async fn test_transaction_timeout() {
        let mvcc_storage = Arc::new(MVCCStorage::new());
        let tx_manager = TransactionManager::new(mvcc_storage);

        let config = TransactionConfig {
            isolation_level: IsolationLevel::ReadCommitted,
            timeout: Duration::from_millis(1), // Very short timeout
            read_only: false,
        };

        let tx_id = tx_manager.begin_transaction(config).await.unwrap();

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Commit should fail due to timeout
        let result = tx_manager.commit_transaction(tx_id).await;
        assert!(result.is_err());
    }
}
