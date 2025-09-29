//! Configuration validation

use crate::ShardForgeConfig;
use shardforge_core::Result;

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate the entire configuration
    pub fn validate(config: &ShardForgeConfig) -> Result<()> {
        Self::validate_cluster(&config.cluster)?;
        Self::validate_node(&config.node)?;
        Self::validate_storage(&config.storage)?;
        Self::validate_network(&config.network)?;
        Self::validate_logging(&config.logging)?;
        Self::validate_monitoring(&config.monitoring)?;

        // Cross-section validation
        Self::validate_cross_section(config)?;

        Ok(())
    }

    fn validate_cluster(cluster: &crate::ClusterSection) -> Result<()> {
        if cluster.name.is_empty() {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Cluster name cannot be empty".to_string(),
            });
        }

        if cluster.name.len() > 63 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Cluster name too long (max 63 characters)".to_string(),
            });
        }

        // Validate cluster name format (DNS label format)
        if !cluster.name.chars().all(|c| c.is_alphanumeric() || c == '-') {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Cluster name contains invalid characters (only alphanumeric and hyphen allowed)".to_string(),
            });
        }

        // Validate data directory
        if !cluster.data_directory.exists() {
            std::fs::create_dir_all(&cluster.data_directory).map_err(|e| {
                shardforge_core::ShardForgeError::Config {
                    message: format!("Cannot create data directory: {}", e),
                }
            })?;
        }

        Ok(())
    }

    fn validate_node(node: &crate::NodeSection) -> Result<()> {
        // Validate bind address
        if node.bind_address.parse::<std::net::SocketAddr>().is_err() {
            return Err(shardforge_core::ShardForgeError::Config {
                message: format!("Invalid bind address: {}", node.bind_address),
            });
        }

        // Validate advertised address if provided
        if let Some(advertised) = &node.advertised_address {
            if advertised.parse::<std::net::SocketAddr>().is_err() {
                return Err(shardforge_core::ShardForgeError::Config {
                    message: format!("Invalid advertised address: {}", advertised),
                });
            }
        }

        // Validate connection limits
        if node.max_connections == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Maximum connections must be greater than 0".to_string(),
            });
        }

        if node.max_connections > 10000 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Maximum connections too high (max 10000)".to_string(),
            });
        }

        // Validate timeouts
        if node.connection_timeout_sec == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Connection timeout must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    fn validate_storage(storage: &crate::StorageSection) -> Result<()> {
        // Validate cache sizes
        if storage.block_cache_size_mb == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Block cache size must be greater than 0".to_string(),
            });
        }

        if storage.write_buffer_size_mb == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Write buffer size must be greater than 0".to_string(),
            });
        }

        // Validate buffer limits
        if storage.max_write_buffer_number == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Maximum write buffer number must be greater than 0".to_string(),
            });
        }

        // Check for reasonable memory usage
        let total_memory_mb = storage.block_cache_size_mb +
                             (storage.write_buffer_size_mb * storage.max_write_buffer_number);

        if total_memory_mb > 1024 * 1024 { // 1TB
            tracing::warn!("Storage memory usage is very high: {} MB", total_memory_mb);
        }

        Ok(())
    }

    fn validate_network(network: &crate::NetworkSection) -> Result<()> {
        // Validate TLS configuration
        if network.tls_enabled {
            if network.certificate_path.is_none() {
                return Err(shardforge_core::ShardForgeError::Config {
                    message: "TLS enabled but certificate path not specified".to_string(),
                });
            }

            if network.private_key_path.is_none() {
                return Err(shardforge_core::ShardForgeError::Config {
                    message: "TLS enabled but private key path not specified".to_string(),
                });
            }

            // Validate certificate paths exist
            if let Some(cert_path) = &network.certificate_path {
                if !cert_path.exists() {
                    return Err(shardforge_core::ShardForgeError::Config {
                        message: format!("Certificate file does not exist: {}", cert_path.display()),
                    });
                }
            }

            if let Some(key_path) = &network.private_key_path {
                if !key_path.exists() {
                    return Err(shardforge_core::ShardForgeError::Config {
                        message: format!("Private key file does not exist: {}", key_path.display()),
                    });
                }
            }

            if let Some(ca_path) = &network.ca_certificate_path {
                if !ca_path.exists() {
                    return Err(shardforge_core::ShardForgeError::Config {
                        message: format!("CA certificate file does not exist: {}", ca_path.display()),
                    });
                }
            }
        }

        // Validate message size
        if network.max_message_size_mb == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Maximum message size must be greater than 0".to_string(),
            });
        }

        if network.max_message_size_mb > 1024 { // 1GB
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Maximum message size too large (max 1024 MB)".to_string(),
            });
        }

        // Validate connection pool
        if network.connection_pool_size == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Connection pool size must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    fn validate_logging(logging: &crate::LoggingSection) -> Result<()> {
        // Validate log file path if specified
        if let Some(file_path) = &logging.file_path {
            // Check if parent directory exists
            if let Some(parent) = file_path.parent() {
                if !parent.exists() {
                    return Err(shardforge_core::ShardForgeError::Config {
                        message: format!("Log file directory does not exist: {}", parent.display()),
                    });
                }
            }
        }

        // Validate file size limits
        if logging.max_file_size_mb == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Maximum log file size must be greater than 0".to_string(),
            });
        }

        if logging.max_files == 0 {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Maximum number of log files must be greater than 0".to_string(),
            });
        }

        Ok(())
    }

    fn validate_monitoring(monitoring: &crate::MonitoringSection) -> Result<()> {
        // Validate metrics bind address
        if monitoring.enabled {
            if monitoring.bind_address.parse::<std::net::SocketAddr>().is_err() {
                return Err(shardforge_core::ShardForgeError::Config {
                    message: format!("Invalid monitoring bind address: {}", monitoring.bind_address),
                });
            }
        }

        // Validate service name
        if monitoring.service_name.is_empty() {
            return Err(shardforge_core::ShardForgeError::Config {
                message: "Monitoring service name cannot be empty".to_string(),
            });
        }

        Ok(())
    }

    fn validate_cross_section(config: &ShardForgeConfig) -> Result<()> {
        // Validate that monitoring and logging are compatible
        if config.monitoring.tracing_enabled && config.logging.level == crate::LogLevel::Error {
            tracing::warn!("Tracing is enabled but log level is Error - traces may not be visible");
        }

        // Validate resource allocation
        let storage_memory_mb = config.storage.block_cache_size_mb +
                               (config.storage.write_buffer_size_mb * config.storage.max_write_buffer_number);

        // Rough estimate: allow up to 80% of system memory
        if let Ok(system_mem_kb) = std::fs::read_to_string("/proc/meminfo")
            .and_then(|content| {
                content.lines()
                    .find(|line| line.starts_with("MemTotal:"))
                    .and_then(|line| line.split_whitespace().nth(1))
                    .and_then(|s| s.parse::<u64>().ok())
                    .ok_or(std::io::ErrorKind::InvalidData)
            }) {
            let system_mem_mb = system_mem_kb / 1024;
            let max_storage_memory = (system_mem_mb * 8) / 10; // 80%

            if storage_memory_mb > max_storage_memory as usize {
                tracing::warn!(
                    "Storage memory usage ({}) MB exceeds 80% of system memory ({}) MB",
                    storage_memory_mb, system_mem_mb
                );
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ShardForgeConfig;

    #[test]
    fn test_validate_default_config() {
        let config = ShardForgeConfig::default();
        assert!(ConfigValidator::validate(&config).is_ok());
    }

    #[test]
    fn test_validate_empty_cluster_name() {
        let mut config = ShardForgeConfig::default();
        config.cluster.name = String::new();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Cluster name cannot be empty"));
    }

    #[test]
    fn test_validate_tls_without_certificates() {
        let mut config = ShardForgeConfig::default();
        config.network.tls_enabled = true;
        // Don't set certificate paths

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("TLS enabled but certificate path not specified"));
    }

    #[test]
    fn test_validate_invalid_bind_address() {
        let mut config = ShardForgeConfig::default();
        config.node.bind_address = "invalid:address:5432".to_string();

        let result = ConfigValidator::validate(&config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Invalid bind address"));
    }
}
