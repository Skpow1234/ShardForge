//! Performance benchmarks for storage operations

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use shardforge_config::StorageEngineType;
use shardforge_core::{Key, Value};
use shardforge_storage::{StorageConfig, StorageEngine, StorageEngineFactory, WriteOperation};
use tempfile::TempDir;

fn bench_storage_operations(c: &mut Criterion) {
    let runtime = tokio::runtime::Runtime::new().unwrap();

    runtime.block_on(async {
        let config = StorageConfig::default();
        let temp_dir = TempDir::new().unwrap();

        let mut engine =
            StorageEngineFactory::create(StorageEngineType::Memory, &config, temp_dir.path())
                .await
                .unwrap();

        bench_single_operations(&engine, c).await;
        bench_batch_operations(&engine, c).await;

        engine.close().await.unwrap();
    });
}

async fn bench_single_operations(engine: &Box<dyn StorageEngine>, c: &mut Criterion) {
    c.bench_function("storage_put_small", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            let key = Key::from_string("bench_key");
            let value = Value::from_string("bench_value");
            runtime.block_on(async {
                black_box(engine.put(key, value).await.unwrap());
            });
        });
    });

    c.bench_function("storage_get_small", |b| {
        // Pre-populate data
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            for i in 0..1000 {
                let key = Key::from_string(&format!("bench_key_{}", i));
                let value = Value::from_string(&format!("bench_value_{}", i));
                engine.put(key, value).await.unwrap();
            }
        });

        b.iter(|| {
            let key = Key::from_string("bench_key_500");
            runtime.block_on(async {
                black_box(engine.get(&key).await.unwrap());
            });
        });
    });

    c.bench_function("storage_put_large", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            let key = Key::from_string("large_key");
            let data = vec![42; 1024 * 10]; // 10KB
            let value = Value::new(&data);
            runtime.block_on(async {
                black_box(engine.put(key, value).await.unwrap());
            });
        });
    });

    c.bench_function("storage_get_large", |b| {
        // Pre-populate large data
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            let key = Key::from_string("large_key");
            let data = vec![42; 1024 * 10]; // 10KB
            let value = Value::new(&data);
            engine.put(key, value).await.unwrap();
        });

        b.iter(|| {
            let key = Key::from_string("large_key");
            runtime.block_on(async {
                black_box(engine.get(&key).await.unwrap());
            });
        });
    });
}

async fn bench_batch_operations(engine: &Box<dyn StorageEngine>, c: &mut Criterion) {
    c.bench_function("storage_batch_write_10", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            let operations: Vec<WriteOperation> = (0..10)
                .map(|i| WriteOperation::Put {
                    key: Key::from_string(&format!("batch_key_{}", i)),
                    value: Value::from_string(&format!("batch_value_{}", i)),
                })
                .collect();

            runtime.block_on(async {
                black_box(engine.batch_write(operations).await.unwrap());
            });
        });
    });

    c.bench_function("storage_batch_write_100", |b| {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        b.iter(|| {
            let operations: Vec<WriteOperation> = (0..100)
                .map(|i| WriteOperation::Put {
                    key: Key::from_string(&format!("batch_key_{}", i)),
                    value: Value::from_string(&format!("batch_value_{}", i)),
                })
                .collect();

            runtime.block_on(async {
                black_box(engine.batch_write(operations).await.unwrap());
            });
        });
    });

    c.bench_function("storage_batch_mixed_50", |b| {
        // Pre-populate some data for deletes
        let runtime = tokio::runtime::Runtime::new().unwrap();
        runtime.block_on(async {
            for i in 0..25 {
                let key = Key::from_string(&format!("mixed_key_{}", i));
                let value = Value::from_string(&format!("mixed_value_{}", i));
                engine.put(key, value).await.unwrap();
            }
        });

        b.iter(|| {
            let mut operations = vec![];

            // Add puts
            for i in 25..50 {
                operations.push(WriteOperation::Put {
                    key: Key::from_string(&format!("mixed_key_{}", i)),
                    value: Value::from_string(&format!("mixed_value_{}", i)),
                });
            }

            // Add deletes
            for i in 0..25 {
                operations.push(WriteOperation::Delete {
                    key: Key::from_string(&format!("mixed_key_{}", i)),
                });
            }

            runtime.block_on(async {
                black_box(engine.batch_write(operations).await.unwrap());
            });
        });
    });
}

criterion_group!(benches, bench_storage_operations);
criterion_main!(benches);
