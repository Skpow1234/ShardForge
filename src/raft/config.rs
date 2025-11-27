//! RAFT configuration

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// RAFT configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RaftConfig {
    /// Minimum election timeout in milliseconds
    pub election_timeout_min: u64,
    
    /// Maximum election timeout in milliseconds
    pub election_timeout_max: u64,
    
    /// Heartbeat interval in milliseconds
    pub heartbeat_interval: u64,
    
    /// Maximum number of entries per AppendEntries RPC
    pub max_append_entries: usize,
    
    /// Enable pre-vote protocol to prevent disruptive elections
    pub enable_pre_vote: bool,
    
    /// Snapshot threshold (take snapshot after this many log entries)
    pub snapshot_threshold: u64,
    
    /// Maximum number of concurrent replication streams
    pub max_replication_lag: u64,
    
    /// Batch size for log replication
    pub replication_batch_size: usize,
}

impl Default for RaftConfig {
    fn default() -> Self {
        Self {
            election_timeout_min: 150,
            election_timeout_max: 300,
            heartbeat_interval: 50,
            max_append_entries: 100,
            enable_pre_vote: true,
            snapshot_threshold: 10000,
            max_replication_lag: 1000,
            replication_batch_size: 100,
        }
    }
}

impl RaftConfig {
    /// Get election timeout as Duration
    pub fn election_timeout(&self) -> Duration {
        let timeout_ms = rand::random::<u64>() 
            % (self.election_timeout_max - self.election_timeout_min)
            + self.election_timeout_min;
        Duration::from_millis(timeout_ms)
    }
    
    /// Get heartbeat interval as Duration
    pub fn heartbeat_interval(&self) -> Duration {
        Duration::from_millis(self.heartbeat_interval)
    }
    
    /// Validate configuration
    pub fn validate(&self) -> crate::raft::Result<()> {
        if self.election_timeout_min >= self.election_timeout_max {
            return Err(crate::raft::RaftError::ConfigError(
                "election_timeout_min must be less than election_timeout_max".to_string()
            ));
        }
        
        if self.heartbeat_interval >= self.election_timeout_min {
            return Err(crate::raft::RaftError::ConfigError(
                "heartbeat_interval must be less than election_timeout_min".to_string()
            ));
        }
        
        if self.max_append_entries == 0 {
            return Err(crate::raft::RaftError::ConfigError(
                "max_append_entries must be greater than 0".to_string()
            ));
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_default_config() {
        let config = RaftConfig::default();
        assert!(config.validate().is_ok());
    }
    
    #[test]
    fn test_election_timeout_range() {
        let config = RaftConfig::default();
        let timeout = config.election_timeout();
        let min = Duration::from_millis(config.election_timeout_min);
        let max = Duration::from_millis(config.election_timeout_max);
        
        assert!(timeout >= min);
        assert!(timeout < max);
    }
    
    #[test]
    fn test_invalid_config() {
        let mut config = RaftConfig::default();
        config.election_timeout_min = 300;
        config.election_timeout_max = 150;
        
        assert!(config.validate().is_err());
    }
}

