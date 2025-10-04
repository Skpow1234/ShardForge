//! Integration tests for storage engines

use shardforge_core::{Key, Value};
use shardforge_storage::{StorageEngine, StorageEngineFactory, StorageConfig, WriteOperation};
use tempfile::TempDir;

#[tokio::test]
async fn test_memory_engine_integration() {
    let config = StorageConfig::default();
    let temp_dir = TempDir::new().unwrap();

    let engine = StorageEngineFactory::create(
        shardforge_config::StorageEngineType::Memory,
        &config,
        temp_dir.path()
    ).await.unwrap();

    // Test basic CRUD operations
    let test_data = generate_test_data(100);

    // Insert all data
    for (key, value) in &test_data {
        engine.put(key.clone(), value.clone()).await.unwrap();
        assert!(engine.exists(key).await.unwrap());
    }

    // Verify all data is retrievable
    for (key, expected_value) in &test_data {
        let retrieved = engine.get(key).await.unwrap();
        assert_eq!(retrieved, Some(expected_value.clone()));
    }

    // Test batch operations
    let batch_ops: Vec<WriteOperation> = test_data.iter()
        .enumerate()
        .filter(|(i, _)| i % 2 == 0) // Delete even-indexed items
        .map(|(_, (key, _))| WriteOperation::Delete { key: key.clone() })
        .collect();

    engine.batch_write(batch_ops).await.unwrap();

    // Verify deletions
    for (i, (key, _)) in test_data.iter().enumerate() {
        let should_exist = i % 2 != 0;
        assert_eq!(engine.exists(key).await.unwrap(), should_exist);
    }

    let stats = engine.stats();
    assert!(stats.write_operations > 0);
    assert!(stats.read_operations > 0);

    engine.close().await.unwrap();
}

#[tokio::test]
async fn test_storage_engine_concurrency() {
    let config = StorageConfig::default();
    let temp_dir = TempDir::new().unwrap();

    let engine = StorageEngineFactory::create(
        shardforge_config::StorageEngineType::Memory,
        &config,
        temp_dir.path()
    ).await.unwrap();

    // Use Arc for proper sharing across tasks
    let shared_engine = std::sync::Arc::new(engine);
    let mut handles = vec![];

    // Spawn multiple concurrent operations
    for i in 0..10 {
        let engine_clone = shared_engine.clone();

        let handle = tokio::spawn(async move {
            for j in 0..100 {
                let key = Key::from_string(&format!("thread_{}_key_{}", i, j));
                let value = Value::from_string(&format!("thread_{}_value_{}", i, j));

                engine_clone.put(key.clone(), value.clone()).await.unwrap();
                let retrieved = engine_clone.get(&key).await.unwrap();
                assert_eq!(retrieved, Some(value));
            }
        });

        handles.push(handle);
    }

    // Wait for all operations to complete
    for handle in handles {
        handle.await.unwrap();
    }

    // No need to close - Arc will handle cleanup automatically
}

#[tokio::test]
async fn test_storage_engine_error_handling() {
    let config = StorageConfig::default();
    let temp_dir = TempDir::new().unwrap();

    let engine = StorageEngineFactory::create(
        shardforge_config::StorageEngineType::Memory,
        &config,
        temp_dir.path()
    ).await.unwrap();

    // Test operations on non-existent keys
    let nonexistent_key = Key::from_string("nonexistent");
    let result = engine.get(&nonexistent_key).await.unwrap();
    assert_eq!(result, None);

    let exists = engine.exists(&nonexistent_key).await.unwrap();
    assert!(!exists);

    // Delete non-existent key should not error
    engine.delete(&nonexistent_key).await.unwrap();

    engine.close().await.unwrap();
}

#[tokio::test]
async fn test_storage_engine_large_data() {
    let config = StorageConfig::default();
    let temp_dir = TempDir::new().unwrap();

    let engine = StorageEngineFactory::create(
        shardforge_config::StorageEngineType::Memory,
        &config,
        temp_dir.path()
    ).await.unwrap();

    // Test with large values
    let large_value = Value::new(vec![42; 1024 * 1024]); // 1MB
    let key = Key::from_string("large_key");

    engine.put(key.clone(), large_value.clone()).await.unwrap();
    let retrieved = engine.get(&key).await.unwrap();
    assert_eq!(retrieved, Some(large_value));

    // Test with many keys
    let mut large_data = vec![];
    for i in 0..1000 {
        let key = Key::from_string(&format!("bulk_key_{}", i));
        let value = Value::from_string(&format!("bulk_value_{}", i));
        large_data.push((key, value));
    }

    for (key, value) in &large_data {
        engine.put(key.clone(), value.clone()).await.unwrap();
    }

    for (key, expected_value) in &large_data {
        let retrieved = engine.get(key).await.unwrap();
        assert_eq!(retrieved, Some(expected_value.clone()));
    }

    engine.close().await.unwrap();
}

#[tokio::test]
async fn test_storage_engine_flush_and_compact() {
    let config = StorageConfig::default();
    let temp_dir = TempDir::new().unwrap();

    let engine = StorageEngineFactory::create(
        shardforge_config::StorageEngineType::Memory,
        &config,
        temp_dir.path()
    ).await.unwrap();

    // Insert some data
    for i in 0..100 {
        let key = Key::from_string(&format!("flush_key_{}", i));
        let value = Value::from_string(&format!("flush_value_{}", i));
        engine.put(key, value).await.unwrap();
    }

    // Flush (no-op for memory engine, but should not error)
    engine.flush().await.unwrap();

    // Compact (no-op for memory engine, but should not error)
    engine.compact().await.unwrap();

    engine.close().await.unwrap();
}

fn generate_test_data(count: usize) -> Vec<(Key, Value)> {
    (0..count)
        .map(|i| {
            let key = Key::from_string(&format!("test_key_{}", i));
            let value = Value::from_string(&format!("test_value_{}", i));
            (key, value)
        })
        .collect()
}
