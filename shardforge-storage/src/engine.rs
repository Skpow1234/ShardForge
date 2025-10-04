//! Storage engine abstraction

use async_trait::async_trait;
use shardforge_config::CompressionType;
use shardforge_core::{Key, Result, Timestamp, Value};
use std::collections::HashMap;

/// Storage engine statistics
#[derive(Debug, Clone, Default)]
pub struct StorageStats {
    /// Total number of keys stored
    pub key_count: u64,
    /// Total size of stored data in bytes
    pub data_size_bytes: u64,
    /// Number of read operations
    pub read_operations: u64,
    /// Number of write operations
    pub write_operations: u64,
    /// Cache hit rate (0.0 to 1.0)
    pub cache_hit_rate: f64,
    /// Compaction time in milliseconds
    pub compaction_time_ms: u64,
}

/// Storage engine configuration
#[derive(Debug, Clone)]
pub struct StorageConfig {
    /// Block cache size in MB
    pub block_cache_size_mb: usize,
    /// Write buffer size in MB
    pub write_buffer_size_mb: usize,
    /// Maximum write buffer number
    pub max_write_buffer_number: usize,
    /// Enable statistics collection
    pub enable_statistics: bool,
    /// Compression algorithm
    pub compression: CompressionType,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            block_cache_size_mb: 256,
            write_buffer_size_mb: 64,
            max_write_buffer_number: 4,
            enable_statistics: true,
            compression: CompressionType::Lz4,
        }
    }
}

/// Write operation for batch writes
#[derive(Debug, Clone)]
pub enum WriteOperation {
    Put { key: Key, value: Value },
    Delete { key: Key },
}

/// Storage engine trait
#[async_trait]
pub trait StorageEngine: Send + Sync {
    /// Get a value by key
    async fn get(&self, key: &Key) -> Result<Option<Value>>;

    /// Put a key-value pair
    async fn put(&self, key: Key, value: Value) -> Result<()>;

    /// Delete a key
    async fn delete(&self, key: &Key) -> Result<()>;

    /// Check if a key exists
    async fn exists(&self, key: &Key) -> Result<bool>;

    /// Batch write operations
    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<()>;

    /// Get storage statistics
    fn stats(&self) -> StorageStats;

    /// Flush all pending writes
    async fn flush(&self) -> Result<()>;

    /// Force compaction
    async fn compact(&self) -> Result<()>;

    /// Close the storage engine
    async fn close(&mut self) -> Result<()>;
}

/// Factory for creating storage engines
pub struct StorageEngineFactory;

impl StorageEngineFactory {
    /// Create a storage engine based on configuration
    pub async fn create(
        engine_type: shardforge_config::StorageEngineType,
        config: &StorageConfig,
        data_path: &std::path::Path,
    ) -> Result<Box<dyn StorageEngine>> {
        match engine_type {
            #[cfg(feature = "rocksdb")]
            shardforge_config::StorageEngineType::RocksDB => {
                let engine = crate::rocksdb::RocksDBEngine::new(config, data_path).await?;
                Ok(Box::new(engine))
            }
            #[cfg(not(feature = "rocksdb"))]
            shardforge_config::StorageEngineType::RocksDB => {
                Err(shardforge_core::ShardForgeError::Config {
                    message: "RocksDB feature not enabled. Enable with --features rocksdb"
                        .to_string(),
                })
            }
            #[cfg(feature = "sled")]
            shardforge_config::StorageEngineType::Sled => {
                let engine = crate::sled::SledEngine::new(config, data_path).await?;
                Ok(Box::new(engine))
            }
            #[cfg(not(feature = "sled"))]
            shardforge_config::StorageEngineType::Sled => {
                Err(shardforge_core::ShardForgeError::Config {
                    message: "Sled feature not enabled. Enable with --features sled".to_string(),
                })
            }
            shardforge_config::StorageEngineType::Memory => {
                let engine = crate::memory::MemoryEngine::new(config).await?;
                Ok(Box::new(engine))
            }
        }
    }
}

/// Transaction handle for multi-version concurrency control
#[derive(Debug)]
pub struct TransactionHandle {
    id: shardforge_core::TransactionId,
    start_timestamp: Timestamp,
    writes: HashMap<Key, Option<Value>>, // None means delete
}

impl TransactionHandle {
    pub fn new(id: shardforge_core::TransactionId, timestamp: Timestamp) -> Self {
        Self {
            id,
            start_timestamp: timestamp,
            writes: HashMap::new(),
        }
    }

    pub fn id(&self) -> &shardforge_core::TransactionId {
        &self.id
    }

    pub fn start_timestamp(&self) -> Timestamp {
        self.start_timestamp
    }

    pub fn add_write(&mut self, key: Key, value: Option<Value>) {
        self.writes.insert(key, value);
    }

    pub fn get_write(&self, key: &Key) -> Option<&Option<Value>> {
        self.writes.get(key)
    }

    pub fn writes(&self) -> &HashMap<Key, Option<Value>> {
        &self.writes
    }

    pub fn take_writes(self) -> HashMap<Key, Option<Value>> {
        self.writes
    }
}
