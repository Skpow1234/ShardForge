//! Multi-Version Concurrency Control (MVCC) implementation

use std::collections::BTreeMap;
use std::sync::Arc;

use shardforge_core::{Key, Result, ShardForgeError, Timestamp, TransactionId, Value};
use tokio::sync::RwLock;

/// Version of a value in MVCC
#[derive(Debug, Clone)]
pub struct Version {
    /// The value at this version
    pub value: Option<Value>, // None means deleted
    /// Transaction that created this version
    pub transaction_id: TransactionId,
    /// Timestamp when this version was created
    pub timestamp: Timestamp,
    /// Whether this version is committed
    pub committed: bool,
    /// Whether this version is visible (not aborted)
    pub visible: bool,
}

impl Version {
    /// Create a new version
    pub fn new(
        value: Option<Value>,
        transaction_id: TransactionId,
        timestamp: Timestamp,
        committed: bool,
    ) -> Self {
        Self {
            value,
            transaction_id,
            timestamp,
            committed,
            visible: true,
        }
    }

    /// Mark this version as aborted
    pub fn abort(&mut self) {
        self.visible = false;
    }

    /// Mark this version as committed
    pub fn commit(&mut self) {
        self.committed = true;
    }

    /// Check if this version is visible to a transaction at a given timestamp
    pub fn is_visible_to(&self, timestamp: Timestamp) -> bool {
        self.visible && self.committed && self.timestamp <= timestamp
    }
}

/// MVCC storage for managing multiple versions of values
pub struct MVCCStorage {
    /// Store versions for each key
    versions: Arc<RwLock<BTreeMap<Key, Vec<Version>>>>,
    /// Global timestamp counter
    timestamp_counter: Arc<RwLock<u64>>,
    /// Active transaction set
    active_transactions: Arc<RwLock<std::collections::HashSet<TransactionId>>>,
}

impl MVCCStorage {
    /// Create a new MVCC storage
    pub fn new() -> Self {
        Self {
            versions: Arc::new(RwLock::new(BTreeMap::new())),
            timestamp_counter: Arc::new(RwLock::new(1)),
            active_transactions: Arc::new(RwLock::new(std::collections::HashSet::new())),
        }
    }

    /// Generate a new timestamp
    pub async fn next_timestamp(&self) -> Timestamp {
        let mut counter = self.timestamp_counter.write().await;
        let timestamp = *counter;
        *counter += 1;
        Timestamp::new(timestamp)
    }

    /// Begin a transaction
    pub async fn begin_transaction(&self, transaction_id: TransactionId) -> Timestamp {
        let timestamp = self.next_timestamp().await;
        let mut transactions = self.active_transactions.write().await;
        transactions.insert(transaction_id);
        timestamp
    }

    /// Commit a transaction
    pub async fn commit_transaction(&self, transaction_id: TransactionId) -> Result<()> {
        let mut versions = self.versions.write().await;
        
        // Mark all versions created by this transaction as committed
        for versions_list in versions.values_mut() {
            for version in versions_list.iter_mut() {
                if version.transaction_id == transaction_id {
                    version.commit();
                }
            }
        }

        // Remove from active transactions
        let mut transactions = self.active_transactions.write().await;
        transactions.remove(&transaction_id);

        Ok(())
    }

    /// Abort a transaction
    pub async fn abort_transaction(&self, transaction_id: TransactionId) -> Result<()> {
        let mut versions = self.versions.write().await;
        
        // Mark all versions created by this transaction as aborted
        for versions_list in versions.values_mut() {
            for version in versions_list.iter_mut() {
                if version.transaction_id == transaction_id {
                    version.abort();
                }
            }
        }

        // Remove from active transactions
        let mut transactions = self.active_transactions.write().await;
        transactions.remove(&transaction_id);

        Ok(())
    }

    /// Read a value at a specific timestamp
    pub async fn read(&self, key: &Key, timestamp: Timestamp) -> Result<Option<Value>> {
        let versions = self.versions.read().await;
        
        if let Some(versions_list) = versions.get(key) {
            // Find the latest visible version at the given timestamp
            for version in versions_list.iter().rev() {
                if version.is_visible_to(timestamp) {
                    return Ok(version.value.clone());
                }
            }
        }

        Ok(None)
    }

    /// Write a value (creates a new version)
    pub async fn write(
        &self,
        key: Key,
        value: Option<Value>,
        transaction_id: TransactionId,
        timestamp: Timestamp,
    ) -> Result<()> {
        let mut versions = self.versions.write().await;
        
        // Check for write-write conflicts
        if let Some(versions_list) = versions.get(&key) {
            for version in versions_list.iter().rev() {
                // If there's an uncommitted write by another transaction after our snapshot
                if version.timestamp > timestamp 
                    && version.transaction_id != transaction_id 
                    && !version.committed {
                    return Err(ShardForgeError::Transaction {
                        message: format!("Write-write conflict detected for key {:?}", key),
                    });
                }
            }
        }

        // Create new version
        let new_version = Version::new(value, transaction_id, timestamp, false);
        
        // Add to versions list
        let versions_list = versions.entry(key).or_insert_with(Vec::new);
        versions_list.push(new_version);
        
        // Sort versions by timestamp to maintain order
        versions_list.sort_by_key(|v| v.timestamp.value());

        Ok(())
    }

    /// Delete a key (creates a delete version)
    pub async fn delete(
        &self,
        key: Key,
        transaction_id: TransactionId,
        timestamp: Timestamp,
    ) -> Result<()> {
        self.write(key, None, transaction_id, timestamp).await
    }

    /// Get all versions for a key (for debugging/testing)
    pub async fn get_versions(&self, key: &Key) -> Vec<Version> {
        let versions = self.versions.read().await;
        if let Some(versions_list) = versions.get(key) {
            versions_list.clone()
        } else {
            Vec::new()
        }
    }

    /// Garbage collect old versions that are not visible to any active transaction
    pub async fn garbage_collect(&self) -> Result<usize> {
        let active_transactions = self.active_transactions.read().await;
        let mut versions = self.versions.write().await;
        let mut collected_count = 0;

        // Find oldest active transaction timestamp
        let oldest_active_timestamp = if active_transactions.is_empty() {
            // If no active transactions, we can clean up everything that's not the latest committed version
            u64::MAX
        } else {
            // For simplicity, we'll be conservative and not collect much when there are active transactions
            0
        };

        for (_key, versions_list) in versions.iter_mut() {
            let mut new_versions = Vec::new();
            let mut latest_committed = None;

            // Keep the latest committed version and any versions newer than oldest active transaction
            for version in versions_list.iter().rev() {
                if version.committed && latest_committed.is_none() {
                    latest_committed = Some(version.clone());
                    new_versions.insert(0, version.clone());
                } else if version.timestamp.value() > oldest_active_timestamp {
                    new_versions.insert(0, version.clone());
                } else {
                    collected_count += 1;
                }
            }

            *versions_list = new_versions;
        }

        Ok(collected_count)
    }

    /// Get storage statistics
    pub async fn stats(&self) -> MVCCStats {
        let versions = self.versions.read().await;
        let active_transactions = self.active_transactions.read().await;
        
        let total_keys = versions.len();
        let total_versions = versions.values().map(|v| v.len()).sum();
        let active_transaction_count = active_transactions.len();

        MVCCStats {
            total_keys,
            total_versions,
            active_transaction_count,
        }
    }
}

/// Statistics for MVCC storage
#[derive(Debug, Clone)]
pub struct MVCCStats {
    /// Total number of keys
    pub total_keys: usize,
    /// Total number of versions across all keys
    pub total_versions: usize,
    /// Number of active transactions
    pub active_transaction_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;
    use shardforge_core::{Key, TransactionId, Value};

    #[tokio::test]
    async fn test_mvcc_basic_read_write() {
        let mvcc = MVCCStorage::new();
        let tx1 = TransactionId::new();
        let key = Key::new(b"test_key");
        let value = Value::new(b"test_value");

        // Begin transaction
        let timestamp = mvcc.begin_transaction(tx1).await;

        // Write value
        mvcc.write(key.clone(), Some(value.clone()), tx1, timestamp).await.unwrap();

        // Read before commit should not see the value (from another transaction's perspective)
        let read_timestamp = mvcc.next_timestamp().await;
        let result = mvcc.read(&key, read_timestamp).await.unwrap();
        assert!(result.is_none());

        // Commit transaction
        mvcc.commit_transaction(tx1).await.unwrap();

        // Read after commit should see the value
        let result = mvcc.read(&key, read_timestamp).await.unwrap();
        assert_eq!(result, Some(value));
    }

    #[tokio::test]
    async fn test_mvcc_write_conflict() {
        let mvcc = MVCCStorage::new();
        let tx1 = TransactionId::new();
        let tx2 = TransactionId::new();
        let key = Key::new(b"test_key");
        let value1 = Value::new(b"value1");
        let value2 = Value::new(b"value2");

        // Begin both transactions
        let timestamp1 = mvcc.begin_transaction(tx1).await;
        let timestamp2 = mvcc.begin_transaction(tx2).await;

        // First transaction writes
        mvcc.write(key.clone(), Some(value1), tx1, timestamp1).await.unwrap();

        // Second transaction tries to write the same key - should conflict
        let result = mvcc.write(key.clone(), Some(value2), tx2, timestamp2).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_mvcc_garbage_collection() {
        let mvcc = MVCCStorage::new();
        let tx1 = TransactionId::new();
        let key = Key::new(b"test_key");
        let value1 = Value::new(b"value1");
        let value2 = Value::new(b"value2");

        // Create multiple versions
        let timestamp1 = mvcc.begin_transaction(tx1).await;
        mvcc.write(key.clone(), Some(value1), tx1, timestamp1).await.unwrap();
        mvcc.commit_transaction(tx1).await.unwrap();

        let tx2 = TransactionId::new();
        let timestamp2 = mvcc.begin_transaction(tx2).await;
        mvcc.write(key.clone(), Some(value2), tx2, timestamp2).await.unwrap();
        mvcc.commit_transaction(tx2).await.unwrap();

        // Before GC, should have 2 versions
        let versions = mvcc.get_versions(&key).await;
        assert_eq!(versions.len(), 2);

        // Run garbage collection
        let collected = mvcc.garbage_collect().await.unwrap();
        
        // After GC, should still have at least 1 version (the latest committed)
        let versions = mvcc.get_versions(&key).await;
        assert!(!versions.is_empty());
    }
}
