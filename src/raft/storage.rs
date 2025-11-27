//! RAFT persistent storage interface

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use super::{LogEntry, LogIndex, PersistentState, Result, Term};

/// Persistent storage interface for RAFT
#[async_trait]
pub trait RaftStorage: Send + Sync {
    /// Save persistent state
    async fn save_state(&self, state: &PersistentState) -> Result<()>;
    
    /// Load persistent state
    async fn load_state(&self) -> Result<Option<PersistentState>>;
    
    /// Append log entries
    async fn append_entries(&self, entries: &[LogEntry]) -> Result<()>;
    
    /// Get log entry at index
    async fn get_entry(&self, index: LogIndex) -> Result<Option<LogEntry>>;
    
    /// Get log entries in range [start, end)
    async fn get_entries(&self, start: LogIndex, end: LogIndex) -> Result<Vec<LogEntry>>;
    
    /// Get last log index
    async fn last_index(&self) -> Result<LogIndex>;
    
    /// Get last log term
    async fn last_term(&self) -> Result<Term>;
    
    /// Truncate log after index
    async fn truncate_after(&self, index: LogIndex) -> Result<()>;
    
    /// Save snapshot metadata
    async fn save_snapshot_metadata(&self, last_index: LogIndex, last_term: Term) -> Result<()>;
    
    /// Load snapshot metadata
    async fn load_snapshot_metadata(&self) -> Result<Option<(LogIndex, Term)>>;
}

/// In-memory storage implementation (for testing)
#[derive(Debug, Clone)]
pub struct MemoryStorage {
    state: std::sync::Arc<tokio::sync::RwLock<MemoryStorageInner>>,
}

#[derive(Debug, Clone, Default)]
struct MemoryStorageInner {
    persistent_state: Option<PersistentState>,
    log_entries: Vec<LogEntry>,
    snapshot_index: LogIndex,
    snapshot_term: Term,
}

impl MemoryStorage {
    pub fn new() -> Self {
        Self {
            state: std::sync::Arc::new(tokio::sync::RwLock::new(MemoryStorageInner::default())),
        }
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl RaftStorage for MemoryStorage {
    async fn save_state(&self, state: &PersistentState) -> Result<()> {
        let mut inner = self.state.write().await;
        inner.persistent_state = Some(state.clone());
        Ok(())
    }
    
    async fn load_state(&self) -> Result<Option<PersistentState>> {
        let inner = self.state.read().await;
        Ok(inner.persistent_state.clone())
    }
    
    async fn append_entries(&self, entries: &[LogEntry]) -> Result<()> {
        let mut inner = self.state.write().await;
        for entry in entries {
            // Handle log overwrites
            if entry.index as usize <= inner.log_entries.len() {
                inner.log_entries.truncate(entry.index as usize - 1);
            }
            inner.log_entries.push(entry.clone());
        }
        Ok(())
    }
    
    async fn get_entry(&self, index: LogIndex) -> Result<Option<LogEntry>> {
        let inner = self.state.read().await;
        if index == 0 || index as usize > inner.log_entries.len() {
            Ok(None)
        } else {
            Ok(Some(inner.log_entries[index as usize - 1].clone()))
        }
    }
    
    async fn get_entries(&self, start: LogIndex, end: LogIndex) -> Result<Vec<LogEntry>> {
        let inner = self.state.read().await;
        
        if start == 0 || start > end {
            return Ok(Vec::new());
        }
        
        let start_idx = (start - 1) as usize;
        let end_idx = (end - 1).min(inner.log_entries.len() as u64) as usize;
        
        if start_idx >= inner.log_entries.len() {
            return Ok(Vec::new());
        }
        
        Ok(inner.log_entries[start_idx..end_idx].to_vec())
    }
    
    async fn last_index(&self) -> Result<LogIndex> {
        let inner = self.state.read().await;
        if inner.log_entries.is_empty() {
            Ok(inner.snapshot_index)
        } else {
            Ok(inner.log_entries.len() as LogIndex)
        }
    }
    
    async fn last_term(&self) -> Result<Term> {
        let inner = self.state.read().await;
        if let Some(last_entry) = inner.log_entries.last() {
            Ok(last_entry.term)
        } else {
            Ok(inner.snapshot_term)
        }
    }
    
    async fn truncate_after(&self, index: LogIndex) -> Result<()> {
        let mut inner = self.state.write().await;
        if index < inner.log_entries.len() as LogIndex {
            inner.log_entries.truncate(index as usize);
        }
        Ok(())
    }
    
    async fn save_snapshot_metadata(&self, last_index: LogIndex, last_term: Term) -> Result<()> {
        let mut inner = self.state.write().await;
        inner.snapshot_index = last_index;
        inner.snapshot_term = last_term;
        
        // Remove log entries covered by snapshot
        if last_index >= inner.log_entries.len() as LogIndex {
            inner.log_entries.clear();
        } else {
            inner.log_entries.drain(..last_index as usize);
        }
        
        Ok(())
    }
    
    async fn load_snapshot_metadata(&self) -> Result<Option<(LogIndex, Term)>> {
        let inner = self.state.read().await;
        if inner.snapshot_index > 0 {
            Ok(Some((inner.snapshot_index, inner.snapshot_term)))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::LogEntry;
    
    #[tokio::test]
    async fn test_save_and_load_state() {
        let storage = MemoryStorage::new();
        
        let state = PersistentState {
            current_term: 5,
            voted_for: Some(2),
            cluster_id: "test".to_string(),
        };
        
        storage.save_state(&state).await.unwrap();
        let loaded = storage.load_state().await.unwrap();
        
        assert!(loaded.is_some());
        let loaded = loaded.unwrap();
        assert_eq!(loaded.current_term, 5);
        assert_eq!(loaded.voted_for, Some(2));
    }
    
    #[tokio::test]
    async fn test_append_and_get_entries() {
        let storage = MemoryStorage::new();
        
        let entries = vec![
            LogEntry::new_command(1, 1, b"cmd1".to_vec()),
            LogEntry::new_command(2, 1, b"cmd2".to_vec()),
        ];
        
        storage.append_entries(&entries).await.unwrap();
        
        let last_idx = storage.last_index().await.unwrap();
        assert_eq!(last_idx, 2);
        
        let entry = storage.get_entry(1).await.unwrap();
        assert!(entry.is_some());
        assert_eq!(entry.unwrap().command, b"cmd1");
    }
    
    #[tokio::test]
    async fn test_get_entries_range() {
        let storage = MemoryStorage::new();
        
        let entries = vec![
            LogEntry::new_command(1, 1, b"cmd1".to_vec()),
            LogEntry::new_command(2, 1, b"cmd2".to_vec()),
            LogEntry::new_command(3, 2, b"cmd3".to_vec()),
        ];
        
        storage.append_entries(&entries).await.unwrap();
        
        let range = storage.get_entries(2, 4).await.unwrap();
        assert_eq!(range.len(), 2);
        assert_eq!(range[0].index, 2);
        assert_eq!(range[1].index, 3);
    }
    
    #[tokio::test]
    async fn test_truncate() {
        let storage = MemoryStorage::new();
        
        let entries = vec![
            LogEntry::new_command(1, 1, b"cmd1".to_vec()),
            LogEntry::new_command(2, 1, b"cmd2".to_vec()),
            LogEntry::new_command(3, 2, b"cmd3".to_vec()),
        ];
        
        storage.append_entries(&entries).await.unwrap();
        storage.truncate_after(2).await.unwrap();
        
        let last_idx = storage.last_index().await.unwrap();
        assert_eq!(last_idx, 2);
    }
    
    #[tokio::test]
    async fn test_snapshot_metadata() {
        let storage = MemoryStorage::new();
        
        storage.save_snapshot_metadata(10, 2).await.unwrap();
        
        let metadata = storage.load_snapshot_metadata().await.unwrap();
        assert!(metadata.is_some());
        
        let (index, term) = metadata.unwrap();
        assert_eq!(index, 10);
        assert_eq!(term, 2);
    }
}

