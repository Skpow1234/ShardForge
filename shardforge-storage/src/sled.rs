//! Sled storage engine implementation

use super::{StorageConfig, StorageEngine, StorageStats, WriteOperation};
use async_trait::async_trait;
use shardforge_core::{Key, Result, Value};
use std::path::Path;

#[cfg(feature = "sled")]
use sled::{Config, Db};

#[cfg(feature = "sled")]
pub struct SledEngine {
    db: Db,
    stats: StorageStats,
}

#[cfg(feature = "sled")]
impl SledEngine {
    pub async fn new(config: &StorageConfig, data_path: &Path) -> Result<Self> {
        let sled_config = Config::new()
            .path(data_path)
            .cache_capacity(config.block_cache_size_mb * 1024 * 1024);

        let db = sled_config
            .open()
            .map_err(|e| shardforge_core::ShardForgeError::Storage { source: Box::new(e) })?;

        Ok(Self { db, stats: StorageStats::default() })
    }
}

#[cfg(feature = "sled")]
#[async_trait]
impl StorageEngine for SledEngine {
    async fn get(&self, key: &Key) -> Result<Option<Value>> {
        let result = self
            .db
            .get(key.as_slice())
            .map_err(|e| shardforge_core::ShardForgeError::Storage { source: Box::new(e) })?;

        Ok(result.map(|ivec| Value::new(ivec.to_vec())))
    }

    async fn put(&self, key: Key, value: Value) -> Result<()> {
        self.db
            .insert(key.as_slice(), value.as_slice())
            .map_err(|e| shardforge_core::ShardForgeError::Storage { source: Box::new(e) })?;
        Ok(())
    }

    async fn delete(&self, key: &Key) -> Result<()> {
        self.db
            .remove(key.as_slice())
            .map_err(|e| shardforge_core::ShardForgeError::Storage { source: Box::new(e) })?;
        Ok(())
    }

    async fn exists(&self, key: &Key) -> Result<bool> {
        let result = self
            .db
            .contains_key(key.as_slice())
            .map_err(|e| shardforge_core::ShardForgeError::Storage { source: Box::new(e) })?;
        Ok(result)
    }

    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<()> {
        let mut batch = sled::Batch::default();

        for operation in operations {
            match operation {
                WriteOperation::Put { key, value } => {
                    batch.insert(key.as_slice(), value.as_slice());
                }
                WriteOperation::Delete { key } => {
                    batch.remove(key.as_slice());
                }
            }
        }

        self.db
            .apply_batch(batch)
            .map_err(|e| shardforge_core::ShardForgeError::Storage { source: Box::new(e) })?;
        Ok(())
    }

    fn stats(&self) -> StorageStats {
        // TODO: Implement proper Sled statistics collection
        self.stats.clone()
    }

    async fn flush(&self) -> Result<()> {
        self.db
            .flush()
            .map_err(|e| shardforge_core::ShardForgeError::Storage { source: Box::new(e) })?;
        Ok(())
    }

    async fn compact(&self) -> Result<()> {
        // Sled doesn't have explicit compaction, but flush ensures consistency
        self.db
            .flush()
            .map_err(|e| shardforge_core::ShardForgeError::Storage { source: Box::new(e) })?;
        Ok(())
    }

    async fn close(self) -> Result<()> {
        // Sled doesn't have explicit close, Db will be dropped
        Ok(())
    }
}

#[cfg(not(feature = "sled"))]
pub struct SledEngine;

#[cfg(not(feature = "sled"))]
impl SledEngine {
    pub async fn new(_config: &StorageConfig, _data_path: &Path) -> Result<Self> {
        Err(shardforge_core::ShardForgeError::Storage {
            source: std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "Sled storage engine not enabled (compile with --features sled)",
            )
            .into(),
        })
    }
}

#[cfg(not(feature = "sled"))]
#[async_trait]
impl StorageEngine for SledEngine {
    async fn get(&self, _key: &Key) -> Result<Option<Value>> {
        unimplemented!("Sled storage engine not enabled")
    }

    async fn put(&self, _key: Key, _value: Value) -> Result<()> {
        unimplemented!("Sled storage engine not enabled")
    }

    async fn delete(&self, _key: &Key) -> Result<()> {
        unimplemented!("Sled storage engine not enabled")
    }

    async fn exists(&self, _key: &Key) -> Result<bool> {
        unimplemented!("Sled storage engine not enabled")
    }

    async fn batch_write(&self, _operations: Vec<WriteOperation>) -> Result<()> {
        unimplemented!("Sled storage engine not enabled")
    }

    fn stats(&self) -> StorageStats {
        unimplemented!("Sled storage engine not enabled")
    }

    async fn flush(&self) -> Result<()> {
        unimplemented!("Sled storage engine not enabled")
    }

    async fn compact(&self) -> Result<()> {
        unimplemented!("Sled storage engine not enabled")
    }

    async fn close(self) -> Result<()> {
        unimplemented!("Sled storage engine not enabled")
    }
}
