//! Integration tests for configuration system

use shardforge_config::*;
use std::env;
use tempfile::TempDir;
use tokio::fs;

#[tokio::test]
async fn test_config_loader_hierarchical_loading() {
    let temp_dir = TempDir::new().unwrap();
    let system_config = temp_dir.path().join("system.toml");
    let user_config = temp_dir.path().join("user.toml");
    let local_config = temp_dir.path().join("local.toml");

    // Create system config
    let system_content = r#"
        [cluster]
        name = "system-cluster"

        [storage]
        engine = "rocksdb"
    "#;
    fs::write(&system_config, system_content).await.unwrap();

    // Create user config
    let user_content = r#"
        [cluster]
        name = "user-cluster"

        [node]
        bind_address = "127.0.0.1:8080"
    "#;
    fs::write(&user_config, user_content).await.unwrap();

    // Create local config
    let local_content = r#"
        [cluster]
        name = "local-cluster"

        [monitoring]
        enabled = false
    "#;
    fs::write(&local_config, local_content).await.unwrap();

    // Set up search paths
    let search_paths = vec![
        system_config.parent().unwrap().to_str().unwrap().to_string(),
        user_config.parent().unwrap().to_str().unwrap().to_string(),
    ];

    let loader = ConfigLoader::new().with_search_paths(search_paths);

    // Change to local config directory
    let original_dir = env::current_dir().unwrap();
    env::set_current_dir(local_config.parent().unwrap()).unwrap();

    let config = loader.load().await.unwrap();

    // Should use local config (highest precedence)
    assert_eq!(config.cluster.name, "local-cluster");
    assert_eq!(config.node.bind_address, "127.0.0.1:8080"); // From user config
    assert_eq!(config.storage.engine, StorageEngineType::RocksDB); // From system config
    assert!(!config.monitoring.enabled); // From local config

    // Restore original directory
    env::set_current_dir(original_dir).unwrap();
}

#[tokio::test]
async fn test_config_loader_environment_variables() {
    // Set environment variables
    env::set_var("SHARDFORGE_CLUSTER__NAME", "env-cluster");
    env::set_var("SHARDFORGE_NODE__BIND_ADDRESS", "0.0.0.0:9999");
    env::set_var("SHARDFORGE_STORAGE__BLOCK_CACHE_SIZE_MB", "512");

    let loader = ConfigLoader::new();
    let config = loader.load().await.unwrap();

    assert_eq!(config.cluster.name, "env-cluster");
    assert_eq!(config.node.bind_address, "0.0.0.0:9999");
    assert_eq!(config.storage.block_cache_size_mb, 512);

    // Clean up
    env::remove_var("SHARDFORGE_CLUSTER__NAME");
    env::remove_var("SHARDFORGE_NODE__BIND_ADDRESS");
    env::remove_var("SHARDFORGE_STORAGE__BLOCK_CACHE_SIZE_MB");
}

#[tokio::test]
async fn test_config_loader_tilde_expansion() {
    if let Some(home) = env::var_os("HOME") {
        let home_str = home.to_string_lossy();

        let config_content = format!(r#"
            [network]
            certificate_path = "~/certs/cert.pem"
            private_key_path = "~/certs/key.pem"
            ca_certificate_path = "~/certs/ca.pem"

            [logging]
            file_path = "~/logs/shardforge.log"
        "#);

        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("tilde.toml");
        fs::write(&config_path, config_content).await.unwrap();

        let loader = ConfigLoader::new();
        let config = loader.load_from_file(&config_path).await.unwrap();

        // Tildes should be expanded
        let expected_cert_path = format!("{}/certs/cert.pem", home_str);
        let expected_key_path = format!("{}/certs/key.pem", home_str);
        let expected_ca_path = format!("{}/certs/ca.pem", home_str);
        let expected_log_path = format!("{}/logs/shardforge.log", home_str);

        assert_eq!(config.network.certificate_path.unwrap().to_str().unwrap(), expected_cert_path);
        assert_eq!(config.network.private_key_path.unwrap().to_str().unwrap(), expected_key_path);
        assert_eq!(config.network.ca_certificate_path.unwrap().to_str().unwrap(), expected_ca_path);
        assert_eq!(config.logging.file_path.unwrap().to_str().unwrap(), expected_log_path);
    }
}

#[tokio::test]
async fn test_config_validation_comprehensive() {
    let temp_dir = TempDir::new().unwrap();

    // Create valid certificates for TLS testing
    let certs_dir = temp_dir.path().join("certs");
    fs::create_dir(&certs_dir).await.unwrap();

    let cert_path = certs_dir.join("cert.pem");
    let key_path = certs_dir.join("key.pem");
    let ca_path = certs_dir.join("ca.pem");

    // Create dummy certificate files
    fs::write(&cert_path, "dummy cert").await.unwrap();
    fs::write(&key_path, "dummy key").await.unwrap();
    fs::write(&ca_path, "dummy ca").await.unwrap();

    let mut config = ShardForgeConfig::default();

    // Configure TLS
    config.network.tls_enabled = true;
    config.network.certificate_path = Some(cert_path.clone());
    config.network.private_key_path = Some(key_path.clone());
    config.network.ca_certificate_path = Some(ca_path.clone());

    // Configure logging
    let log_dir = temp_dir.path().join("logs");
    fs::create_dir(&log_dir).await.unwrap();
    config.logging.file_path = Some(log_dir.join("shardforge.log"));

    // Configure data directory
    let data_dir = temp_dir.path().join("data");
    fs::create_dir(&data_dir).await.unwrap();
    config.cluster.data_directory = data_dir;

    // This should pass validation
    assert!(ConfigValidator::validate(&config).is_ok());

    // Test invalid configurations
    config.network.tls_enabled = true;
    config.network.certificate_path = None;
    assert!(ConfigValidator::validate(&config).is_err());

    config.network.certificate_path = Some(cert_path);
    config.network.private_key_path = None;
    assert!(ConfigValidator::validate(&config).is_err());

    // Reset TLS config
    config.network.private_key_path = Some(key_path);
    config.network.tls_enabled = false;

    // Test invalid cluster name
    config.cluster.name = "invalid@name!".to_string();
    assert!(ConfigValidator::validate(&config).is_err());

    config.cluster.name = "valid-name".to_string();

    // Test invalid bind address
    config.node.bind_address = "invalid:address".to_string();
    assert!(ConfigValidator::validate(&config).is_err());
}

#[tokio::test]
async fn test_config_round_trip() {
    let original_config = ShardForgeConfig::default();
    original_config.cluster.name = "round-trip-test".to_string();
    original_config.node.bind_address = "127.0.0.1:9999".to_string();

    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("round_trip.toml");

    let loader = ConfigLoader::new();
    loader.save_to_file(&original_config, &config_path).await.unwrap();

    let loaded_config = loader.load_from_file(&config_path).await.unwrap();

    assert_eq!(loaded_config.cluster.name, original_config.cluster.name);
    assert_eq!(loaded_config.node.bind_address, original_config.node.bind_address);
}

#[tokio::test]
async fn test_config_loader_nonexistent_files() {
    let loader = ConfigLoader::new();

    // Should succeed with default config even if files don't exist
    let config = loader.load().await.unwrap();
    assert_eq!(config.cluster.name, "default-cluster");
}

#[tokio::test]
async fn test_config_validation_edge_cases() {
    // Test minimum valid configuration
    let mut config = ShardForgeConfig::default();
    config.storage.block_cache_size_mb = 1;
    config.storage.write_buffer_size_mb = 1;
    config.storage.max_write_buffer_number = 1;
    config.network.max_message_size_mb = 1;
    config.logging.max_file_size_mb = 1;
    config.logging.max_files = 1;

    assert!(ConfigValidator::validate(&config).is_ok());

    // Test zero values (should fail)
    config.storage.block_cache_size_mb = 0;
    assert!(ConfigValidator::validate(&config).is_err());

    config.storage.block_cache_size_mb = 1;
    config.network.max_message_size_mb = 0;
    assert!(ConfigValidator::validate(&config).is_err());

    config.network.max_message_size_mb = 1;
    config.logging.max_files = 0;
    assert!(ConfigValidator::validate(&config).is_err());
}
