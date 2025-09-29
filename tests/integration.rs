//! Full system integration tests

use shardforge_core::*;
use shardforge_config::*;
use shardforge_storage::*;
use tempfile::TempDir;

#[tokio::test]
async fn test_full_system_integration() {
    // Create temporary directories
    let temp_dir = TempDir::new().unwrap();
    let data_dir = temp_dir.path().join("data");
    std::fs::create_dir(&data_dir).unwrap();

    // 1. Load configuration
    let config_content = r#"
        [cluster]
        name = "integration-test-cluster"
        data_directory = "${DATA_DIR}"

        [node]
        bind_address = "127.0.0.1:0"  # Use port 0 for auto-assignment

        [storage]
        engine = "memory"

        [logging]
        level = "debug"
    "#.replace("${DATA_DIR}", data_dir.to_str().unwrap());

    let config_path = temp_dir.path().join("integration.toml");
    tokio::fs::write(&config_path, config_content).await.unwrap();

    let loader = ConfigLoader::new();
    let config = loader.load_from_file(&config_path).await.unwrap();

    assert_eq!(config.cluster.name, "integration-test-cluster");
    assert_eq!(config.storage.engine, StorageEngineType::Memory);

    // 2. Create storage engine
    let storage_config = StorageConfig {
        block_cache_size_mb: config.storage.block_cache_size_mb,
        write_buffer_size_mb: config.storage.write_buffer_size_mb,
        max_write_buffer_number: config.storage.max_write_buffer_number,
        enable_statistics: config.storage.enable_statistics,
        compression: match config.storage.compression {
            crate::config::CompressionType::None => CompressionType::None,
            crate::config::CompressionType::Snappy => CompressionType::Snappy,
            crate::config::CompressionType::Lz4 => CompressionType::Lz4,
            crate::config::CompressionType::Zstd => CompressionType::Zstd,
        },
    };

    let engine = StorageEngineFactory::create(
        config.storage.engine,
        &storage_config,
        &data_dir
    ).await.unwrap();

    // 3. Test full CRUD operations
    let test_cases = vec![
        ("user:1", r#"{"name": "Alice", "age": 30}"#),
        ("user:2", r#"{"name": "Bob", "age": 25}"#),
        ("product:1", r#"{"name": "Widget", "price": 19.99}"#),
        ("order:1", r#"{"user_id": 1, "product_id": 1, "quantity": 2}"#),
    ];

    // Insert test data
    for (key_str, value_str) in &test_cases {
        let key = Key::from_string(key_str);
        let value = Value::from_string(value_str);
        engine.put(key, value).await.unwrap();
    }

    // Verify data integrity
    for (key_str, expected_value_str) in &test_cases {
        let key = Key::from_string(key_str);
        let retrieved = engine.get(&key).await.unwrap();
        assert!(retrieved.is_some());

        let expected_value = Value::from_string(expected_value_str);
        assert_eq!(retrieved.unwrap(), expected_value);
    }

    // Test batch operations
    let batch_ops = vec![
        WriteOperation::Put {
            key: Key::from_string("batch:key1"),
            value: Value::from_string("batch_value_1"),
        },
        WriteOperation::Put {
            key: Key::from_string("batch:key2"),
            value: Value::from_string("batch_value_2"),
        },
        WriteOperation::Delete {
            key: Key::from_string("user:1"),
        },
    ];

    engine.batch_write(batch_ops).await.unwrap();

    // Verify batch operations
    assert_eq!(engine.get(&Key::from_string("batch:key1")).await.unwrap(),
               Some(Value::from_string("batch_value_1")));
    assert_eq!(engine.get(&Key::from_string("batch:key2")).await.unwrap(),
               Some(Value::from_string("batch_value_2")));
    assert_eq!(engine.get(&Key::from_string("user:1")).await.unwrap(), None);

    // Test statistics
    let stats = engine.stats();
    assert!(stats.write_operations >= 7); // 4 initial + 3 batch
    assert!(stats.read_operations >= 5); // 4 verification + 3 batch check

    // 4. Test system lifecycle
    engine.flush().await.unwrap();
    engine.compact(None).await.unwrap();
    engine.close().await.unwrap();

    println!("✅ Full system integration test passed!");
}

#[tokio::test]
async fn test_concurrent_workload_simulation() {
    let temp_dir = TempDir::new().unwrap();

    let config = StorageConfig::default();
    let engine = StorageEngineFactory::create(
        shardforge_config::StorageEngineType::Memory,
        &config,
        temp_dir.path()
    ).await.unwrap();

    let num_workers = 10;
    let operations_per_worker = 100;

    let mut handles = vec![];

    // Spawn worker tasks
    for worker_id in 0..num_workers {
        let engine_ref = match &*engine {
            StorageEngineType::Memory(mem) => mem.clone(),
            _ => panic!("Test only supports memory engine"),
        };

        let handle = tokio::spawn(async move {
            let mut local_stats = (0, 0); // (writes, reads)

            for op_id in 0..operations_per_worker {
                let key = Key::from_string(&format!("worker_{}_op_{}", worker_id, op_id));
                let value = Value::from_string(&format!("data_{}_{}", worker_id, op_id));

                // Write operation
                engine_ref.put(key.clone(), value.clone()).await.unwrap();
                local_stats.0 += 1;

                // Read operation
                let retrieved = engine_ref.get(&key).await.unwrap();
                assert_eq!(retrieved, Some(value));
                local_stats.1 += 1;

                // Small delay to simulate real workload
                tokio::time::sleep(tokio::time::Duration::from_micros(10)).await;
            }

            local_stats
        });

        handles.push(handle);
    }

    // Collect results
    let mut total_writes = 0;
    let mut total_reads = 0;

    for handle in handles {
        let (writes, reads) = handle.await.unwrap();
        total_writes += writes;
        total_reads += reads;
    }

    // Verify totals
    assert_eq!(total_writes, num_workers * operations_per_worker);
    assert_eq!(total_reads, num_workers * operations_per_worker);

    // Verify global statistics
    let stats = engine.stats();
    assert!(stats.write_operations >= total_writes as u64);
    assert!(stats.read_operations >= total_reads as u64);

    engine.close().await.unwrap();

    println!("✅ Concurrent workload simulation test passed!");
}

#[tokio::test]
async fn test_error_handling_and_recovery() {
    let temp_dir = TempDir::new().unwrap();

    let config = StorageConfig::default();
    let engine = StorageEngineFactory::create(
        shardforge_config::StorageEngineType::Memory,
        &config,
        temp_dir.path()
    ).await.unwrap();

    // Test graceful handling of various error conditions
    let nonexistent_key = Key::from_string("nonexistent");

    // Should not error on non-existent key operations
    assert_eq!(engine.get(&nonexistent_key).await.unwrap(), None);
    assert!(!engine.exists(&nonexistent_key).await.unwrap());
    engine.delete(&nonexistent_key).await.unwrap();

    // Test with large data
    let large_key = Key::new(vec![1; 1024]); // 1KB key
    let large_value = Value::new(vec![2; 1024 * 1024]); // 1MB value

    engine.put(large_key.clone(), large_value.clone()).await.unwrap();
    let retrieved = engine.get(&large_key).await.unwrap();
    assert_eq!(retrieved, Some(large_value));

    // Test batch operations with mixed valid/invalid operations
    let batch_ops = vec![
        WriteOperation::Put {
            key: Key::from_string("valid_key"),
            value: Value::from_string("valid_value"),
        },
        WriteOperation::Delete {
            key: Key::from_string("nonexistent_key"),
        },
    ];

    engine.batch_write(batch_ops).await.unwrap();

    // Verify results
    assert_eq!(engine.get(&Key::from_string("valid_key")).await.unwrap(),
               Some(Value::from_string("valid_value")));

    engine.close().await.unwrap();

    println!("✅ Error handling and recovery test passed!");
}

#[tokio::test]
async fn test_system_performance_baseline() {
    let temp_dir = TempDir::new().unwrap();

    let config = StorageConfig::default();
    let engine = StorageEngineFactory::create(
        shardforge_config::StorageEngineType::Memory,
        &config,
        temp_dir.path()
    ).await.unwrap();

    let num_operations = 1000;

    // Measure write performance
    let write_start = std::time::Instant::now();
    for i in 0..num_operations {
        let key = Key::from_string(&format!("perf_write_{}", i));
        let value = Value::from_string(&format!("value_{}", i));
        engine.put(key, value).await.unwrap();
    }
    let write_duration = write_start.elapsed();

    // Measure read performance
    let read_start = std::time::Instant::now();
    for i in 0..num_operations {
        let key = Key::from_string(&format!("perf_write_{}", i));
        let _ = engine.get(&key).await.unwrap();
    }
    let read_duration = read_start.elapsed();

    // Calculate rates
    let write_rate = num_operations as f64 / write_duration.as_secs_f64();
    let read_rate = num_operations as f64 / read_duration.as_secs_f64();

    println!("Performance baseline:");
    println!("  Writes: {:.0} ops/sec", write_rate);
    println!("  Reads:  {:.0} ops/sec", read_rate);

    // Basic performance assertions (adjust based on system capabilities)
    assert!(write_rate > 100.0, "Write performance too low: {:.0} ops/sec", write_rate);
    assert!(read_rate > 1000.0, "Read performance too low: {:.0} ops/sec", read_rate);

    engine.close().await.unwrap();

    println!("✅ System performance baseline test passed!");
}
