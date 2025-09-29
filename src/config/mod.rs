//! Configuration management and validation

pub mod config;
pub mod loader;
pub mod validator;

/// Re-export commonly used types
pub use config::*;
pub use loader::*;
pub use validator::*;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = ShardForgeConfig::default();

        assert_eq!(config.cluster.name, "default-cluster");
        assert_eq!(config.node.bind_address, "127.0.0.1:5432");
        assert!(!config.network.tls_enabled);
        assert_eq!(config.storage.engine, StorageEngineType::RocksDB);
        assert_eq!(config.logging.level, LogLevel::Info);
        assert!(config.monitoring.enabled);
    }

    #[test]
    fn test_config_validation_success() {
        let config = ShardForgeConfig::default();
        assert!(ConfigValidator::validate(&config).is_ok());
    }

    #[test]
    fn test_config_validation_cluster_name() {
        let mut config = ShardForgeConfig::default();
        config.cluster.name = String::new();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Cluster name cannot be empty"));
    }

    #[test]
    fn test_config_validation_cluster_name_length() {
        let mut config = ShardForgeConfig::default();
        config.cluster.name = "a".repeat(64); // Too long

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Cluster name too long"));
    }

    #[test]
    fn test_config_validation_cluster_name_chars() {
        let mut config = ShardForgeConfig::default();
        config.cluster.name = "invalid@name!".to_string();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("invalid characters"));
    }

    #[test]
    fn test_config_validation_bind_address() {
        let mut config = ShardForgeConfig::default();
        config.node.bind_address = "invalid:address:5432".to_string();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Invalid bind address"));
    }

    #[test]
    fn test_config_validation_storage_cache() {
        let mut config = ShardForgeConfig::default();
        config.storage.block_cache_size_mb = 0;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Block cache size must be greater than 0"));
    }

    #[test]
    fn test_config_validation_tls_certificates() {
        let mut config = ShardForgeConfig::default();
        config.network.tls_enabled = true;
        // Don't set certificate paths

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("TLS enabled but certificate path not specified"));
    }

    #[test]
    fn test_config_validation_message_size() {
        let mut config = ShardForgeConfig::default();
        config.network.max_message_size_mb = 0;

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Maximum message size must be greater than 0"));
    }

    #[test]
    fn test_config_validation_log_file() {
        let temp_dir = TempDir::new().unwrap();
        let mut config = ShardForgeConfig::default();
        config.logging.file_path = Some(temp_dir.path().join("nonexistent").join("log.txt"));

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        let err_str = result.unwrap_err().to_string();
        assert!(err_str.contains("Log file directory does not exist"));
    }

    #[tokio::test]
    async fn test_config_loader_default() {
        let loader = ConfigLoader::new();
        let config = loader.load().await.unwrap();

        assert_eq!(config.cluster.name, "default-cluster");
        assert_eq!(config.node.bind_address, "127.0.0.1:5432");
    }

    #[tokio::test]
    async fn test_config_loader_from_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test.toml");

        let config_content = r#"
            [cluster]
            name = "test-cluster"

            [node]
            bind_address = "127.0.0.1:8080"

            [storage]
            engine = "memory"
        "#;

        tokio::fs::write(&config_path, config_content).await.unwrap();

        let loader = ConfigLoader::new();
        let config = loader.load_from_file(&config_path).await.unwrap();

        assert_eq!(config.cluster.name, "test-cluster");
        assert_eq!(config.node.bind_address, "127.0.0.1:8080");
        assert_eq!(config.storage.engine, StorageEngineType::Memory);
    }

    #[tokio::test]
    async fn test_config_save_and_load() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("saved.toml");

        let mut original_config = ShardForgeConfig::default();
        original_config.cluster.name = "saved-cluster".to_string();

        let loader = ConfigLoader::new();
        loader.save_to_file(&original_config, &config_path).await.unwrap();

        let loaded_config = loader.load_from_file(&config_path).await.unwrap();

        assert_eq!(loaded_config.cluster.name, "saved-cluster");
    }

    #[test]
    fn test_config_display_traits() {
        assert_eq!(format!("{}", StorageEngineType::RocksDB), "rocksdb");
        assert_eq!(format!("{}", StorageEngineType::Memory), "memory");
        assert_eq!(format!("{}", StorageEngineType::Sled), "sled");

        assert_eq!(format!("{}", CompressionType::None), "none");
        assert_eq!(format!("{}", CompressionType::Lz4), "lz4");
        assert_eq!(format!("{}", CompressionType::Zstd), "zstd");

        assert_eq!(format!("{}", LogLevel::Trace), "trace");
        assert_eq!(format!("{}", LogLevel::Info), "info");
        assert_eq!(format!("{}", LogLevel::Error), "error");

        assert_eq!(format!("{}", LogFormat::Json), "json");
        assert_eq!(format!("{}", LogFormat::Pretty), "pretty");
    }
}
