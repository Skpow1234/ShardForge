//! Storage engine abstraction and implementations

// Storage modules (to be implemented)
// pub mod engine;
// pub mod iterator;
// pub mod memory;
// pub mod rocksdb;
// pub mod sled;

// Re-export commonly used types (when modules are implemented)
// pub use engine::*;
// pub use iterator::*;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{Key, Value};

    #[tokio::test]
    async fn test_storage_config_defaults() {
        let config = StorageConfig::default();
        assert_eq!(config.block_cache_size_mb, 256);
        assert_eq!(config.write_buffer_size_mb, 64);
        assert_eq!(config.max_write_buffer_number, 4);
        assert!(config.enable_statistics);
        assert_eq!(config.compression, CompressionType::Lz4);
    }

    #[tokio::test]
    async fn test_write_operation_variants() {
        let key = Key::from_string("test_key");
        let value = Value::from_string("test_value");

        let put_op = WriteOperation::Put { key: key.clone(), value: value.clone() };
        let delete_op = WriteOperation::Delete { key: key.clone() };

        match put_op {
            WriteOperation::Put { key: k, value: v } => {
                assert_eq!(k, key);
                assert_eq!(v, value);
            }
            _ => panic!("Wrong operation type"),
        }

        match delete_op {
            WriteOperation::Delete { key: k } => {
                assert_eq!(k, key);
            }
            _ => panic!("Wrong operation type"),
        }
    }

    #[tokio::test]
    async fn test_iterator_options() {
        let start_key = Key::from_string("key1");
        let end_key = Key::from_string("key5");

        // Test range options
        let range_opts = IteratorOptions::range(start_key.clone(), end_key.clone());
        assert!(range_opts.should_include(&Key::from_string("key2")));
        assert!(range_opts.should_include(&Key::from_string("key4")));
        assert!(!range_opts.should_include(&Key::from_string("key0")));
        assert!(!range_opts.should_include(&Key::from_string("key6")));

        // Test inclusive end
        let mut inclusive_opts = range_opts;
        inclusive_opts.inclusive_end = true;
        assert!(inclusive_opts.should_include(&end_key));

        // Test from options
        let from_opts = IteratorOptions::from(start_key.clone());
        assert!(from_opts.should_include(&Key::from_string("key2")));
        assert!(!from_opts.should_include(&Key::from_string("key0")));

        // Test all options
        let all_opts = IteratorOptions::all();
        assert!(all_opts.should_include(&Key::from_string("any_key")));

        // Test with limit
        let limited_opts = IteratorOptions::all().with_limit(10);
        assert_eq!(limited_opts.limit, Some(10));

        // Test with direction
        let backward_opts = IteratorOptions::all().with_direction(IterationDirection::Backward);
        assert_eq!(backward_opts.direction, IterationDirection::Backward);
    }

    #[tokio::test]
    async fn test_storage_factory_memory() {
        let config = StorageConfig::default();
        let temp_dir = tempfile::temp_dir().unwrap();

        // Test memory engine creation
        let engine = StorageEngineFactory::create(
            crate::config::StorageEngineType::Memory,
            &config,
            temp_dir.path()
        ).await.unwrap();

        // Test basic operations
        let key = Key::from_string("test_key");
        let value = Value::from_string("test_value");

        engine.put(key.clone(), value.clone()).await.unwrap();
        let retrieved = engine.get(&key).await.unwrap();
        assert_eq!(retrieved, Some(value));

        assert!(engine.exists(&key).await.unwrap());
        engine.delete(&key).await.unwrap();
        assert!(!engine.exists(&key).await.unwrap());

        let stats = engine.stats();
        assert!(stats.read_operations >= 1);
        assert!(stats.write_operations >= 2);

        engine.close().await.unwrap();
    }

    #[tokio::test]
    async fn test_batch_write_operations() {
        let config = StorageConfig::default();
        let temp_dir = tempfile::temp_dir().unwrap();

        let engine = StorageEngineFactory::create(
            crate::config::StorageEngineType::Memory,
            &config,
            temp_dir.path()
        ).await.unwrap();

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

        // Key1 should not exist (deleted)
        assert_eq!(engine.get(&Key::from_string("key1")).await.unwrap(), None);

        // Key2 should exist
        assert_eq!(
            engine.get(&Key::from_string("key2")).await.unwrap(),
            Some(Value::from_string("value2"))
        );

        engine.close().await.unwrap();
    }
}
