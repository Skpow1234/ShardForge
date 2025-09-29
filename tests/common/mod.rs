//! Common test utilities and helpers

use shardforge_core::*;
use shardforge_config::*;
use shardforge_storage::*;
use tempfile::TempDir;
use std::sync::Arc;

/// Test environment setup
pub struct TestEnv {
    pub temp_dir: TempDir,
    pub config: ShardForgeConfig,
    pub storage_engine: Box<dyn StorageEngine>,
}

impl TestEnv {
    /// Create a new test environment with default settings
    pub async fn new() -> Result<Self> {
        Self::with_config(ShardForgeConfig::default()).await
    }

    /// Create a new test environment with custom configuration
    pub async fn with_config(mut config: ShardForgeConfig) -> Result<Self> {
        let temp_dir = TempDir::new().map_err(|e| {
            ShardForgeError::Internal(format!("Failed to create temp dir: {}", e))
        })?;

        // Override config to use temp directory
        config.cluster.data_directory = temp_dir.path().to_path_buf();
        config.storage.engine = StorageEngineType::Memory; // Use memory engine for tests

        // Create storage engine
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

        let storage_engine = StorageEngineFactory::create(
            config.storage.engine,
            &storage_config,
            &config.cluster.data_directory,
        ).await?;

        Ok(Self {
            temp_dir,
            config,
            storage_engine,
        })
    }

    /// Generate test key-value pairs
    pub fn generate_test_data(&self, count: usize) -> Vec<(Key, Value)> {
        (0..count)
            .map(|i| {
                let key = Key::from_string(&format!("test_key_{}", i));
                let value = Value::from_string(&format!("test_value_{}", i));
                (key, value)
            })
            .collect()
    }

    /// Populate storage with test data
    pub async fn populate_test_data(&self, count: usize) -> Result<Vec<(Key, Value)>> {
        let test_data = self.generate_test_data(count);

        for (key, value) in &test_data {
            self.storage_engine.put(key.clone(), value.clone()).await?;
        }

        Ok(test_data)
    }

    /// Verify data integrity
    pub async fn verify_data_integrity(&self, data: &[(Key, Value)]) -> Result<()> {
        for (key, expected_value) in data {
            let retrieved = self.storage_engine.get(key).await?;
            match retrieved {
                Some(actual_value) => {
                    if actual_value != *expected_value {
                        return Err(ShardForgeError::Internal(format!(
                            "Data mismatch for key {:?}: expected {:?}, got {:?}",
                            key, expected_value, actual_value
                        )));
                    }
                }
                None => {
                    return Err(ShardForgeError::Internal(format!(
                        "Key {:?} not found", key
                    )));
                }
            }
        }

        Ok(())
    }

    /// Clean up the test environment
    pub async fn cleanup(self) -> Result<()> {
        self.storage_engine.close().await?;
        // TempDir will be automatically cleaned up when dropped
        Ok(())
    }
}

/// Performance measurement utilities
pub struct PerformanceTimer {
    start: std::time::Instant,
    label: String,
}

impl PerformanceTimer {
    pub fn new(label: &str) -> Self {
        Self {
            start: std::time::Instant::now(),
            label: label.to_string(),
        }
    }

    pub fn elapsed(&self) -> std::time::Duration {
        self.start.elapsed()
    }

    pub fn print(&self) {
        let elapsed = self.elapsed();
        println!("{}: {:.2}ms", self.label, elapsed.as_millis());
    }

    pub fn finish(self) -> std::time::Duration {
        let elapsed = self.elapsed();
        println!("{}: {:.2}ms", self.label, elapsed.as_millis());
        elapsed
    }
}

/// Macro for timing code blocks
#[macro_export]
macro_rules! time_block {
    ($label:expr, $code:block) => {{
        let _timer = $crate::common::PerformanceTimer::new($label);
        let result = $code;
        _timer.print();
        result
    }};
}

/// Test data generators
pub struct TestDataGenerator;

impl TestDataGenerator {
    /// Generate sequential keys and values
    pub fn sequential(count: usize) -> Vec<(Key, Value)> {
        (0..count)
            .map(|i| {
                let key = Key::from_string(&format!("seq_key_{:08}", i));
                let value = Value::from_string(&format!("seq_value_{:08}", i));
                (key, value)
            })
            .collect()
    }

    /// Generate random keys and values
    pub fn random(count: usize) -> Vec<(Key, Value)> {
        use rand::{Rng, thread_rng};

        let mut rng = thread_rng();
        (0..count)
            .map(|i| {
                let key_data: Vec<u8> = (0..16).map(|_| rng.gen()).collect();
                let value_data: Vec<u8> = (0..64).map(|_| rng.gen()).collect();
                let key = Key::new(key_data);
                let value = Value::new(value_data);
                (key, value)
            })
            .collect()
    }

    /// Generate keys with a common prefix
    pub fn prefixed(prefix: &str, count: usize) -> Vec<(Key, Value)> {
        (0..count)
            .map(|i| {
                let key = Key::from_string(&format!("{}{:08}", prefix, i));
                let value = Value::from_string(&format!("prefixed_value_{}", i));
                (key, value)
            })
            .collect()
    }

    /// Generate large values for stress testing
    pub fn large_values(count: usize, value_size_kb: usize) -> Vec<(Key, Value)> {
        (0..count)
            .map(|i| {
                let key = Key::from_string(&format!("large_key_{}", i));
                let value_data = vec![42u8; value_size_kb * 1024];
                let value = Value::new(value_data);
                (key, value)
            })
            .collect()
    }
}

/// Concurrent test utilities
pub struct ConcurrentTestRunner;

impl ConcurrentTestRunner {
    /// Run a test function concurrently with multiple workers
    pub async fn run_concurrent<F, Fut>(
        worker_count: usize,
        operations_per_worker: usize,
        test_fn: F,
    ) -> Result<()>
    where
        F: Fn(usize, usize) -> Fut + Send + Sync + Clone + 'static,
        Fut: std::future::Future<Output = Result<()>> + Send,
    {
        let mut handles = vec![];

        for worker_id in 0..worker_count {
            let test_fn_clone = test_fn.clone();

            let handle = tokio::spawn(async move {
                for op_id in 0..operations_per_worker {
                    test_fn_clone(worker_id, op_id).await?;
                }
                Ok::<(), ShardForgeError>(())
            });

            handles.push(handle);
        }

        // Wait for all workers to complete
        for handle in handles {
            handle.await.map_err(|e| {
                ShardForgeError::Internal(format!("Task panicked: {}", e))
            })??;
        }

        Ok(())
    }
}

/// Memory usage monitoring
pub struct MemoryMonitor {
    initial_memory: usize,
}

impl MemoryMonitor {
    pub fn new() -> Self {
        Self {
            initial_memory: Self::get_current_memory(),
        }
    }

    pub fn get_current_memory() -> usize {
        // Simple memory estimation - in a real implementation,
        // you'd use platform-specific APIs
        0 // Placeholder
    }

    pub fn memory_delta(&self) -> isize {
        Self::get_current_memory() as isize - self.initial_memory as isize
    }

    pub fn print_memory_usage(&self, label: &str) {
        let delta = self.memory_delta();
        println!("{}: {} bytes", label, delta);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_test_env_creation() {
        let env = TestEnv::new().await.unwrap();
        assert!(env.temp_dir.path().exists());
        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_data_generation() {
        let env = TestEnv::new().await.unwrap();

        let test_data = env.generate_test_data(10);
        assert_eq!(test_data.len(), 10);

        for (i, (key, value)) in test_data.iter().enumerate() {
            let expected_key = format!("test_key_{}", i);
            let expected_value = format!("test_value_{}", i);

            assert_eq!(key, &Key::from_string(&expected_key));
            assert_eq!(value, &Value::from_string(&expected_value));
        }

        env.cleanup().await.unwrap();
    }

    #[tokio::test]
    async fn test_data_population_and_verification() {
        let env = TestEnv::new().await.unwrap();

        let test_data = env.populate_test_data(5).await.unwrap();
        env.verify_data_integrity(&test_data).await.unwrap();

        env.cleanup().await.unwrap();
    }

    #[test]
    fn test_performance_timer() {
        let timer = PerformanceTimer::new("test_timer");
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = timer.finish();

        assert!(duration >= std::time::Duration::from_millis(10));
    }

    #[test]
    fn test_test_data_generators() {
        let sequential = TestDataGenerator::sequential(3);
        assert_eq!(sequential.len(), 3);
        assert_eq!(sequential[0].0, Key::from_string("seq_key_00000000"));

        let random = TestDataGenerator::random(3);
        assert_eq!(random.len(), 3);
        // Random data should have different keys
        assert_ne!(random[0].0, random[1].0);

        let prefixed = TestDataGenerator::prefixed("prefix_", 2);
        assert_eq!(prefixed.len(), 2);
        assert!(prefixed[0].0.as_slice().starts_with(b"prefix_"));
    }
}
