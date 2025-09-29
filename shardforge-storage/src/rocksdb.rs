//! RocksDB storage engine implementation

use async_trait::async_trait;
use shardforge_core::{Key, Value, Result};
use std::path::Path;
use super::{StorageEngine, StorageConfig, StorageStats, WriteOperation};

#[cfg(feature = "rocksdb")]
use rocksdb::{DB, Options, WriteBatch, IteratorMode};

#[cfg(feature = "rocksdb")]
pub struct RocksDBEngine {
    db: DB,
    stats: StorageStats,
}

#[cfg(feature = "rocksdb")]
impl RocksDBEngine {
    pub async fn new(config: &StorageConfig, data_path: &Path) -> Result<Self> {
        let mut options = Options::default();

        // Configure RocksDB options
        options.create_if_missing(true);
        options.set_max_write_buffer_number(config.max_write_buffer_number as i32);
        options.set_write_buffer_size(config.write_buffer_size_mb * 1024 * 1024);
        options.set_block_cache_size_mb(config.block_cache_size_mb);

        // Enable compression if configured
        match config.compression {
            super::CompressionType::None => {
                // No compression
            }
            super::CompressionType::Snappy => {
                options.set_compression_type(rocksdb::DBCompressionType::Snappy);
            }
            super::CompressionType::Lz4 => {
                options.set_compression_type(rocksdb::DBCompressionType::Lz4);
            }
            super::CompressionType::Zstd => {
                options.set_compression_type(rocksdb::DBCompressionType::Zstd);
            }
        }

        let db = DB::open(&options, data_path).map_err(|e| {
            shardforge_core::ShardForgeError::Storage {
                source: Box::new(e),
            }
        })?;

        Ok(Self {
            db,
            stats: StorageStats::default(),
        })
    }
}

#[cfg(feature = "rocksdb")]
#[async_trait]
impl StorageEngine for RocksDBEngine {
    async fn get(&self, key: &Key) -> Result<Option<Value>> {
        // Note: RocksDB operations are synchronous, but we implement the async trait
        let result = self.db.get(key.as_slice()).map_err(|e| {
            shardforge_core::ShardForgeError::Storage {
                source: Box::new(e),
            }
        })?;

        Ok(result.map(Value::new))
    }

    async fn put(&self, key: Key, value: Value) -> Result<()> {
        self.db.put(key.as_slice(), value.as_slice()).map_err(|e| {
            shardforge_core::ShardForgeError::Storage {
                source: Box::new(e),
            }
        })?;
        Ok(())
    }

    async fn delete(&self, key: &Key) -> Result<()> {
        self.db.delete(key.as_slice()).map_err(|e| {
            shardforge_core::ShardForgeError::Storage {
                source: Box::new(e),
            }
        })?;
        Ok(())
    }

    async fn exists(&self, key: &Key) -> Result<bool> {
        let result = self.db.get(key.as_slice()).map_err(|e| {
            shardforge_core::ShardForgeError::Storage {
                source: Box::new(e),
            }
        })?;
        Ok(result.is_some())
    }

    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<()> {
        let mut batch = WriteBatch::default();

        for operation in operations {
            match operation {
                WriteOperation::Put { key, value } => {
                    batch.put(key.as_slice(), value.as_slice());
                }
                WriteOperation::Delete { key } => {
                    batch.delete(key.as_slice());
                }
            }
        }

        self.db.write(batch).map_err(|e| {
            shardforge_core::ShardForgeError::Storage {
                source: Box::new(e),
            }
        })?;
        Ok(())
    }

    fn stats(&self) -> StorageStats {
        // TODO: Implement proper RocksDB statistics collection
        self.stats.clone()
    }

    async fn flush(&self) -> Result<()> {
        // RocksDB flush is synchronous
        self.db.flush().map_err(|e| {
            shardforge_core::ShardForgeError::Storage {
                source: Box::new(e),
            }
        })?;
        Ok(())
    }

    async fn compact(&self) -> Result<()> {
        // Compact the entire database
        self.db.compact_range(None::<&[u8]>, None::<&[u8]>);
        Ok(())
    }

    async fn close(self) -> Result<()> {
        // RocksDB doesn't have explicit close, DB will be dropped
        Ok(())
    }
}

#[cfg(not(feature = "rocksdb"))]
pub struct RocksDBEngine;

#[cfg(not(feature = "rocksdb"))]
impl RocksDBEngine {
    pub async fn new(_config: &StorageConfig, _data_path: &Path) -> Result<Self> {
        Err(shardforge_core::ShardForgeError::Storage {
            source: std::io::Error::new(
                std::io::ErrorKind::Unsupported,
                "RocksDB storage engine not enabled (compile with --features rocksdb)"
            ).into(),
        })
    }
}

#[cfg(not(feature = "rocksdb"))]
#[async_trait]
impl StorageEngine for RocksDBEngine {
    async fn get(&self, _key: &Key) -> Result<Option<Value>> {
        unimplemented!("RocksDB storage engine not enabled")
    }

    async fn put(&self, _key: Key, _value: Value) -> Result<()> {
        unimplemented!("RocksDB storage engine not enabled")
    }

    async fn delete(&self, _key: &Key) -> Result<()> {
        unimplemented!("RocksDB storage engine not enabled")
    }

    async fn exists(&self, _key: &Key) -> Result<bool> {
        unimplemented!("RocksDB storage engine not enabled")
    }

    async fn batch_write(&self, _operations: Vec<WriteOperation>) -> Result<()> {
        unimplemented!("RocksDB storage engine not enabled")
    }

    fn stats(&self) -> StorageStats {
        unimplemented!("RocksDB storage engine not enabled")
    }

    async fn flush(&self) -> Result<()> {
        unimplemented!("RocksDB storage engine not enabled")
    }

    async fn compact(&self) -> Result<()> {
        unimplemented!("RocksDB storage engine not enabled")
    }

    async fn close(self) -> Result<()> {
        unimplemented!("RocksDB storage engine not enabled")
    }
}
