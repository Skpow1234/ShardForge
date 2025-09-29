//! In-memory storage engine for testing

use async_trait::async_trait;
use shardforge_core::{Key, Value, Result};
use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use super::{StorageEngine, StorageConfig, StorageStats, WriteOperation};

/// In-memory storage engine for testing and development
pub struct MemoryEngine {
    data: Arc<RwLock<BTreeMap<Key, Value>>>,
    stats: Arc<RwLock<StorageStats>>,
    config: StorageConfig,
}

impl MemoryEngine {
    pub async fn new(_config: &StorageConfig) -> Result<Self> {
        Ok(Self {
            data: Arc::new(RwLock::new(BTreeMap::new())),
            stats: Arc::new(RwLock::new(StorageStats::default())),
            config: _config.clone(),
        })
    }
}

#[async_trait]
impl StorageEngine for MemoryEngine {
    async fn get(&self, key: &Key) -> Result<Option<Value>> {
        let mut stats = self.stats.write().await;
        stats.read_operations += 1;

        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }

    async fn put(&self, key: Key, value: Value) -> Result<()> {
        let mut stats = self.stats.write().await;
        stats.write_operations += 1;

        let mut data = self.data.write().await;
        data.insert(key, value);
        Ok(())
    }

    async fn delete(&self, key: &Key) -> Result<()> {
        let mut stats = self.stats.write().await;
        stats.write_operations += 1;

        let mut data = self.data.write().await;
        data.remove(key);
        Ok(())
    }

    async fn exists(&self, key: &Key) -> Result<bool> {
        let data = self.data.read().await;
        Ok(data.contains_key(key))
    }

    async fn batch_write(&self, operations: Vec<WriteOperation>) -> Result<()> {
        let mut stats = self.stats.write().await;
        stats.write_operations += operations.len() as u64;

        let mut data = self.data.write().await;
        for operation in operations {
            match operation {
                WriteOperation::Put { key, value } => {
                    data.insert(key, value);
                }
                WriteOperation::Delete { key } => {
                    data.remove(&key);
                }
            }
        }
        Ok(())
    }

    fn stats(&self) -> StorageStats {
        // Note: This is a snapshot, stats may change after this call
        futures::executor::block_on(async {
            self.stats.read().await.clone()
        })
    }

    async fn flush(&self) -> Result<()> {
        // No-op for in-memory storage
        Ok(())
    }

    async fn compact(&self) -> Result<()> {
        // No-op for in-memory storage
        Ok(())
    }

    async fn close(self) -> Result<()> {
        // No-op for in-memory storage
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_engine_basic_operations() {
        let config = StorageConfig::default();
        let engine = MemoryEngine::new(&config).await.unwrap();

        let key = Key::from_string("test_key");
        let value = Value::from_string("test_value");

        // Test put and get
        engine.put(key.clone(), value.clone()).await.unwrap();
        let retrieved = engine.get(&key).await.unwrap();
        assert_eq!(retrieved, Some(value));

        // Test exists
        assert!(engine.exists(&key).await.unwrap());

        // Test delete
        engine.delete(&key).await.unwrap();
        assert!(!engine.exists(&key).await.unwrap());
        assert_eq!(engine.get(&key).await.unwrap(), None);
    }

    #[tokio::test]
    async fn test_memory_engine_batch_write() {
        let config = StorageConfig::default();
        let engine = MemoryEngine::new(&config).await.unwrap();

        let operations = vec![
            WriteOperation::Put {
                key: Key::from_string("key1"),
                value: Value::from_string("value1"),
            },
            WriteOperation::Put {
                key: Key::from_string("key2"),
                value: Value::from_string("value2"),
            },
            WriteOperation::Delete {
                key: Key::from_string("key1"),
            },
        ];

        engine.batch_write(operations).await.unwrap();

        assert_eq!(engine.get(&Key::from_string("key1")).await.unwrap(), None);
        assert_eq!(
            engine.get(&Key::from_string("key2")).await.unwrap(),
            Some(Value::from_string("value2"))
        );
    }

    #[tokio::test]
    async fn test_memory_engine_stats() {
        let config = StorageConfig::default();
        let engine = MemoryEngine::new(&config).await.unwrap();

        let key = Key::from_string("test_key");
        let value = Value::from_string("test_value");

        // Initial stats should be zero
        let stats = engine.stats();
        assert_eq!(stats.read_operations, 0);
        assert_eq!(stats.write_operations, 0);

        // Perform operations
        engine.put(key.clone(), value.clone()).await.unwrap();
        engine.get(&key).await.unwrap();
        engine.delete(&key).await.unwrap();

        // Check stats
        let stats = engine.stats();
        assert_eq!(stats.read_operations, 1);
        assert_eq!(stats.write_operations, 2); // put + delete
    }
}
