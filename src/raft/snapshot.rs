//! RAFT snapshot management

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::{LogIndex, Result, Term};

/// Snapshot metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotMetadata {
    /// Last included index in the snapshot
    pub last_included_index: LogIndex,
    
    /// Last included term
    pub last_included_term: Term,
    
    /// Configuration at the time of snapshot
    pub configuration: Vec<u64>,
    
    /// Snapshot creation timestamp
    pub created_at: u64,
    
    /// Snapshot size in bytes
    pub size: u64,
}

/// Snapshot manager for creating and installing snapshots
#[derive(Debug, Clone)]
pub struct SnapshotManager {
    /// Directory for storing snapshots
    snapshot_dir: PathBuf,
    
    /// Current snapshot metadata
    metadata: Option<SnapshotMetadata>,
}

impl SnapshotManager {
    /// Create a new snapshot manager
    pub fn new(snapshot_dir: PathBuf) -> Self {
        Self {
            snapshot_dir,
            metadata: None,
        }
    }
    
    /// Get the current snapshot metadata
    pub fn metadata(&self) -> Option<&SnapshotMetadata> {
        self.metadata.as_ref()
    }
    
    /// Create a new snapshot
    pub async fn create_snapshot(
        &mut self,
        last_index: LogIndex,
        last_term: Term,
        state: &[u8],
    ) -> Result<SnapshotMetadata> {
        // Create snapshot metadata
        let metadata = SnapshotMetadata {
            last_included_index: last_index,
            last_included_term: last_term,
            configuration: Vec::new(),
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            size: state.len() as u64,
        };
        
        // Save snapshot to disk (simplified)
        let snapshot_path = self.snapshot_path(&metadata);
        tokio::fs::create_dir_all(&self.snapshot_dir).await?;
        tokio::fs::write(&snapshot_path, state).await?;
        
        // Save metadata
        let metadata_path = self.metadata_path(&metadata);
        let metadata_bytes = bincode::serialize(&metadata).map_err(|e| {
            super::RaftError::SnapshotError(format!("Failed to serialize metadata: {}", e))
        })?;
        tokio::fs::write(&metadata_path, metadata_bytes).await?;
        
        self.metadata = Some(metadata.clone());
        
        Ok(metadata)
    }
    
    /// Load the latest snapshot
    pub async fn load_latest_snapshot(&mut self) -> Result<Option<(SnapshotMetadata, Vec<u8>)>> {
        // Find latest snapshot by scanning directory
        let mut entries = tokio::fs::read_dir(&self.snapshot_dir).await?;
        let mut latest_metadata: Option<SnapshotMetadata> = None;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("meta") {
                let metadata_bytes = tokio::fs::read(&path).await?;
                if let Ok(metadata) = bincode::deserialize::<SnapshotMetadata>(&metadata_bytes) {
                    if let Some(ref current) = latest_metadata {
                        if metadata.last_included_index > current.last_included_index {
                            latest_metadata = Some(metadata);
                        }
                    } else {
                        latest_metadata = Some(metadata);
                    }
                }
            }
        }
        
        if let Some(metadata) = latest_metadata {
            let snapshot_path = self.snapshot_path(&metadata);
            let data = tokio::fs::read(&snapshot_path).await?;
            self.metadata = Some(metadata.clone());
            Ok(Some((metadata, data)))
        } else {
            Ok(None)
        }
    }
    
    /// Install a snapshot received from leader
    pub async fn install_snapshot(
        &mut self,
        metadata: SnapshotMetadata,
        data: Vec<u8>,
    ) -> Result<()> {
        // Validate snapshot
        if data.len() as u64 != metadata.size {
            return Err(super::RaftError::SnapshotError(
                "Snapshot size mismatch".to_string()
            ));
        }
        
        // Save snapshot
        let snapshot_path = self.snapshot_path(&metadata);
        tokio::fs::create_dir_all(&self.snapshot_dir).await?;
        tokio::fs::write(&snapshot_path, &data).await?;
        
        // Save metadata
        let metadata_path = self.metadata_path(&metadata);
        let metadata_bytes = bincode::serialize(&metadata).map_err(|e| {
            super::RaftError::SnapshotError(format!("Failed to serialize metadata: {}", e))
        })?;
        tokio::fs::write(&metadata_path, metadata_bytes).await?;
        
        self.metadata = Some(metadata);
        
        Ok(())
    }
    
    /// Get the path for a snapshot
    fn snapshot_path(&self, metadata: &SnapshotMetadata) -> PathBuf {
        self.snapshot_dir
            .join(format!("snapshot-{}-{}.snap", metadata.last_included_index, metadata.last_included_term))
    }
    
    /// Get the path for snapshot metadata
    fn metadata_path(&self, metadata: &SnapshotMetadata) -> PathBuf {
        self.snapshot_dir
            .join(format!("snapshot-{}-{}.meta", metadata.last_included_index, metadata.last_included_term))
    }
    
    /// Delete old snapshots (keep only the latest N)
    pub async fn cleanup_old_snapshots(&self, keep_count: usize) -> Result<()> {
        let mut snapshots = Vec::new();
        let mut entries = tokio::fs::read_dir(&self.snapshot_dir).await?;
        
        while let Some(entry) = entries.next_entry().await? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("meta") {
                let metadata_bytes = tokio::fs::read(&path).await?;
                if let Ok(metadata) = bincode::deserialize::<SnapshotMetadata>(&metadata_bytes) {
                    snapshots.push(metadata);
                }
            }
        }
        
        // Sort by index (newest first)
        snapshots.sort_by(|a, b| b.last_included_index.cmp(&a.last_included_index));
        
        // Delete old snapshots
        for metadata in snapshots.iter().skip(keep_count) {
            let snapshot_path = self.snapshot_path(metadata);
            let metadata_path = self.metadata_path(metadata);
            
            tokio::fs::remove_file(snapshot_path).await.ok();
            tokio::fs::remove_file(metadata_path).await.ok();
        }
        
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    
    #[tokio::test]
    async fn test_create_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SnapshotManager::new(temp_dir.path().to_path_buf());
        
        let state = b"test state data";
        let metadata = manager.create_snapshot(10, 2, state).await.unwrap();
        
        assert_eq!(metadata.last_included_index, 10);
        assert_eq!(metadata.last_included_term, 2);
        assert_eq!(metadata.size, state.len() as u64);
    }
    
    #[tokio::test]
    async fn test_load_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SnapshotManager::new(temp_dir.path().to_path_buf());
        
        let state = b"test state data";
        manager.create_snapshot(10, 2, state).await.unwrap();
        
        let loaded = manager.load_latest_snapshot().await.unwrap();
        assert!(loaded.is_some());
        
        let (metadata, data) = loaded.unwrap();
        assert_eq!(metadata.last_included_index, 10);
        assert_eq!(data, state);
    }
    
    #[tokio::test]
    async fn test_install_snapshot() {
        let temp_dir = TempDir::new().unwrap();
        let mut manager = SnapshotManager::new(temp_dir.path().to_path_buf());
        
        let metadata = SnapshotMetadata {
            last_included_index: 15,
            last_included_term: 3,
            configuration: Vec::new(),
            created_at: 0,
            size: 10,
        };
        
        let data = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
        
        manager.install_snapshot(metadata.clone(), data.clone()).await.unwrap();
        
        let loaded = manager.load_latest_snapshot().await.unwrap();
        assert!(loaded.is_some());
        
        let (loaded_meta, loaded_data) = loaded.unwrap();
        assert_eq!(loaded_meta.last_included_index, 15);
        assert_eq!(loaded_data, data);
    }
}

