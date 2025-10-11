//! MVCC support for transaction layer

use std::collections::HashMap;

use shardforge_core::{Result, ShardForgeError, Timestamp, TransactionId};

/// MVCC transaction context
pub struct MVCCTransactionContext {
    /// Transaction ID
    pub transaction_id: TransactionId,
    /// Start timestamp for consistent reads
    pub start_timestamp: Timestamp,
    /// Read timestamp (may be different from start for some isolation levels)
    pub read_timestamp: Timestamp,
    /// Commit timestamp (set during commit)
    pub commit_timestamp: Option<Timestamp>,
}

/// MVCC conflict detector
pub struct ConflictDetector {
    /// Active transactions and their read/write sets
    active_transactions: HashMap<TransactionId, TransactionReadWriteSet>,
}

/// Read/Write set for a transaction
#[derive(Debug, Clone)]
pub struct TransactionReadWriteSet {
    /// Keys read by this transaction
    pub read_set: HashMap<String, Timestamp>,
    /// Keys written by this transaction
    pub write_set: HashMap<String, Timestamp>,
}

/// Conflict types
#[derive(Debug, Clone, PartialEq)]
pub enum ConflictType {
    /// Write-Write conflict
    WriteWrite,
    /// Read-Write conflict
    ReadWrite,
    /// Write-Read conflict (phantom reads)
    WriteRead,
}

/// Conflict information
#[derive(Debug, Clone)]
pub struct Conflict {
    /// Type of conflict
    pub conflict_type: ConflictType,
    /// Resource that caused the conflict
    pub resource: String,
    /// Conflicting transactions
    pub transactions: (TransactionId, TransactionId),
    /// Timestamps involved
    pub timestamps: (Timestamp, Timestamp),
}

impl MVCCTransactionContext {
    /// Create a new MVCC transaction context
    pub fn new(transaction_id: TransactionId, start_timestamp: Timestamp) -> Self {
        Self {
            transaction_id,
            start_timestamp,
            read_timestamp: start_timestamp,
            commit_timestamp: None,
        }
    }

    /// Set commit timestamp
    pub fn set_commit_timestamp(&mut self, timestamp: Timestamp) {
        self.commit_timestamp = Some(timestamp);
    }

    /// Check if this transaction can see a write from another transaction
    pub fn can_see_write(&self, write_timestamp: Timestamp, writer_id: TransactionId) -> bool {
        // Can see own writes
        if writer_id == self.transaction_id {
            return true;
        }

        // Can see writes that happened before our snapshot
        write_timestamp <= self.read_timestamp
    }

    /// Check if a timestamp is in the transaction's read window
    pub fn is_in_read_window(&self, timestamp: Timestamp) -> bool {
        timestamp <= self.read_timestamp
    }
}

impl ConflictDetector {
    /// Create a new conflict detector
    pub fn new() -> Self {
        Self { active_transactions: HashMap::new() }
    }

    /// Add a transaction to tracking
    pub fn add_transaction(&mut self, transaction_id: TransactionId) {
        self.active_transactions.insert(
            transaction_id,
            TransactionReadWriteSet {
                read_set: HashMap::new(),
                write_set: HashMap::new(),
            },
        );
    }

    /// Remove a transaction from tracking
    pub fn remove_transaction(&mut self, transaction_id: TransactionId) {
        self.active_transactions.remove(&transaction_id);
    }

    /// Record a read operation
    pub fn record_read(
        &mut self,
        transaction_id: TransactionId,
        key: String,
        timestamp: Timestamp,
    ) {
        if let Some(rw_set) = self.active_transactions.get_mut(&transaction_id) {
            rw_set.read_set.insert(key, timestamp);
        }
    }

    /// Record a write operation
    pub fn record_write(
        &mut self,
        transaction_id: TransactionId,
        key: String,
        timestamp: Timestamp,
    ) {
        if let Some(rw_set) = self.active_transactions.get_mut(&transaction_id) {
            rw_set.write_set.insert(key, timestamp);
        }
    }

    /// Check for conflicts when a transaction wants to commit
    pub fn check_conflicts(&self, transaction_id: TransactionId) -> Result<Vec<Conflict>> {
        let mut conflicts = Vec::new();

        let committing_rw_set = self.active_transactions.get(&transaction_id).ok_or_else(|| {
            ShardForgeError::Transaction {
                message: format!("Transaction {:?} not found in conflict detector", transaction_id),
            }
        })?;

        // Check for conflicts with other active transactions
        for (&other_tx_id, other_rw_set) in &self.active_transactions {
            if other_tx_id == transaction_id {
                continue;
            }

            // Check for Write-Write conflicts
            for (key, timestamp) in &committing_rw_set.write_set {
                if let Some(other_timestamp) = other_rw_set.write_set.get(key) {
                    conflicts.push(Conflict {
                        conflict_type: ConflictType::WriteWrite,
                        resource: key.clone(),
                        transactions: (transaction_id, other_tx_id),
                        timestamps: (*timestamp, *other_timestamp),
                    });
                }
            }

            // Check for Read-Write conflicts
            for (key, timestamp) in &committing_rw_set.write_set {
                if let Some(other_timestamp) = other_rw_set.read_set.get(key) {
                    // Conflict if the other transaction read the key before we wrote it
                    if *other_timestamp < *timestamp {
                        conflicts.push(Conflict {
                            conflict_type: ConflictType::ReadWrite,
                            resource: key.clone(),
                            transactions: (transaction_id, other_tx_id),
                            timestamps: (*timestamp, *other_timestamp),
                        });
                    }
                }
            }

            // Check for Write-Read conflicts (anti-dependencies)
            for (key, timestamp) in &committing_rw_set.read_set {
                if let Some(other_timestamp) = other_rw_set.write_set.get(key) {
                    // Conflict if the other transaction wrote after we read
                    if *other_timestamp > *timestamp {
                        conflicts.push(Conflict {
                            conflict_type: ConflictType::WriteRead,
                            resource: key.clone(),
                            transactions: (transaction_id, other_tx_id),
                            timestamps: (*timestamp, *other_timestamp),
                        });
                    }
                }
            }
        }

        Ok(conflicts)
    }

    /// Check if a specific operation would cause a conflict
    pub fn would_conflict(
        &self,
        transaction_id: TransactionId,
        key: &str,
        is_write: bool,
        timestamp: Timestamp,
    ) -> bool {
        for (&other_tx_id, other_rw_set) in &self.active_transactions {
            if other_tx_id == transaction_id {
                continue;
            }

            if is_write {
                // Check for write conflicts with other writes or reads
                if other_rw_set.write_set.contains_key(key) {
                    return true; // Write-Write conflict
                }
                if let Some(&read_timestamp) = other_rw_set.read_set.get(key) {
                    if read_timestamp > timestamp {
                        return true; // Read-Write conflict
                    }
                }
            } else {
                // Check for read conflicts with other writes
                if let Some(&write_timestamp) = other_rw_set.write_set.get(key) {
                    if write_timestamp < timestamp {
                        return true; // Write-Read conflict
                    }
                }
            }
        }

        false
    }

    /// Get the number of active transactions
    pub fn active_transaction_count(&self) -> usize {
        self.active_transactions.len()
    }

    /// Get all active transaction IDs
    pub fn active_transactions(&self) -> Vec<TransactionId> {
        self.active_transactions.keys().cloned().collect()
    }
}

impl Default for ConflictDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionReadWriteSet {
    /// Create a new empty read/write set
    pub fn new() -> Self {
        Self {
            read_set: HashMap::new(),
            write_set: HashMap::new(),
        }
    }

    /// Check if this transaction has read a key
    pub fn has_read(&self, key: &str) -> bool {
        self.read_set.contains_key(key)
    }

    /// Check if this transaction has written a key
    pub fn has_written(&self, key: &str) -> bool {
        self.write_set.contains_key(key)
    }

    /// Get all keys read by this transaction
    pub fn read_keys(&self) -> Vec<String> {
        self.read_set.keys().cloned().collect()
    }

    /// Get all keys written by this transaction
    pub fn write_keys(&self) -> Vec<String> {
        self.write_set.keys().cloned().collect()
    }
}

impl Default for TransactionReadWriteSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mvcc_transaction_context() {
        let tx_id = TransactionId::new();
        let start_timestamp = Timestamp::new(100);
        let mut context = MVCCTransactionContext::new(tx_id, start_timestamp);

        assert_eq!(context.transaction_id, tx_id);
        assert_eq!(context.start_timestamp, start_timestamp);
        assert_eq!(context.read_timestamp, start_timestamp);
        assert!(context.commit_timestamp.is_none());

        // Test can_see_write
        assert!(context.can_see_write(Timestamp::new(50), TransactionId::new())); // Earlier write
        assert!(!context.can_see_write(Timestamp::new(150), TransactionId::new())); // Later write
        assert!(context.can_see_write(Timestamp::new(150), tx_id)); // Own write

        // Set commit timestamp
        let commit_timestamp = Timestamp::new(200);
        context.set_commit_timestamp(commit_timestamp);
        assert_eq!(context.commit_timestamp, Some(commit_timestamp));
    }

    #[test]
    fn test_conflict_detector() {
        let mut detector = ConflictDetector::new();
        let tx1 = TransactionId::new();
        let tx2 = TransactionId::new();

        // Add transactions
        detector.add_transaction(tx1);
        detector.add_transaction(tx2);
        assert_eq!(detector.active_transaction_count(), 2);

        // Record operations
        detector.record_read(tx1, "key1".to_string(), Timestamp::new(100));
        detector.record_write(tx2, "key1".to_string(), Timestamp::new(110));

        // Check for conflicts
        let conflicts = detector.check_conflicts(tx1).unwrap();
        // tx1 should have a WriteRead conflict because tx2 wrote after tx1 read
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::WriteRead);

        let conflicts = detector.check_conflicts(tx2).unwrap();
        // tx2 should have a ReadWrite conflict because tx1 read before tx2 wrote
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::ReadWrite);

        // Remove transaction
        detector.remove_transaction(tx1);
        assert_eq!(detector.active_transaction_count(), 1);
    }

    #[test]
    fn test_write_write_conflict() {
        let mut detector = ConflictDetector::new();
        let tx1 = TransactionId::new();
        let tx2 = TransactionId::new();

        detector.add_transaction(tx1);
        detector.add_transaction(tx2);

        // Both transactions write to the same key
        detector.record_write(tx1, "key1".to_string(), Timestamp::new(100));
        detector.record_write(tx2, "key1".to_string(), Timestamp::new(110));

        let conflicts = detector.check_conflicts(tx1).unwrap();
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[0].conflict_type, ConflictType::WriteWrite);
    }

    #[test]
    fn test_would_conflict() {
        let mut detector = ConflictDetector::new();
        let tx1 = TransactionId::new();
        let tx2 = TransactionId::new();

        detector.add_transaction(tx1);
        detector.add_transaction(tx2);

        detector.record_write(tx1, "key1".to_string(), Timestamp::new(100));

        // tx2 trying to write to the same key would conflict
        assert!(detector.would_conflict(tx2, "key1", true, Timestamp::new(110)));

        // tx2 trying to write to a different key would not conflict
        assert!(!detector.would_conflict(tx2, "key2", true, Timestamp::new(110)));

        // tx2 trying to read from the same key at an earlier timestamp would not conflict
        assert!(!detector.would_conflict(tx2, "key1", false, Timestamp::new(90)));
    }
}
