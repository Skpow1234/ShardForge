//! RAFT node implementation

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{mpsc, RwLock};
use tokio::time::{interval, sleep};
use tracing::{debug, error, info, warn};

use super::{
    AppendEntriesRequest, AppendEntriesResponse, InstallSnapshotRequest,
    InstallSnapshotResponse, LeaderState, LogEntry, LogIndex, NodeId, PreVoteState,
    PersistentState, RaftConfig, RaftError, RaftLog, RaftStorage, RequestVoteRequest,
    RequestVoteResponse, Result, Role, SnapshotManager, Term, VolatileState,
};

/// RAFT node
pub struct RaftNode<S: RaftStorage> {
    /// Node ID
    id: NodeId,
    
    /// Persistent state
    persistent_state: Arc<RwLock<PersistentState>>,
    
    /// Volatile state
    volatile_state: Arc<RwLock<VolatileState>>,
    
    /// Leader-specific state (only valid when leader)
    leader_state: Arc<RwLock<Option<LeaderState>>>,
    
    /// RAFT log
    log: Arc<RwLock<RaftLog>>,
    
    /// Persistent storage
    storage: Arc<S>,
    
    /// Configuration
    config: RaftConfig,
    
    /// Cluster members (node IDs and addresses)
    peers: Arc<RwLock<HashMap<NodeId, String>>>,
    
    /// Snapshot manager
    snapshot_manager: Arc<RwLock<SnapshotManager>>,
    
    /// Pre-vote state
    pre_vote_state: Arc<RwLock<PreVoteState>>,
    
    /// Shutdown signal
    shutdown_tx: Option<mpsc::Sender<()>>,
}

impl<S: RaftStorage + 'static> RaftNode<S> {
    /// Create a new RAFT node
    pub async fn new(
        id: NodeId,
        storage: Arc<S>,
        config: RaftConfig,
        snapshot_dir: std::path::PathBuf,
    ) -> Result<Self> {
        config.validate()?;
        
        // Load persistent state from storage
        let persistent_state = if let Some(state) = storage.load_state().await? {
            state
        } else {
            PersistentState::default()
        };
        
        // Initialize log
        let mut log = RaftLog::new();
        
        // Load snapshot metadata
        let snapshot_manager = SnapshotManager::new(snapshot_dir);
        if let Some((last_index, last_term)) = storage.load_snapshot_metadata().await? {
            log.install_snapshot(last_index, last_term);
        }
        
        Ok(Self {
            id,
            persistent_state: Arc::new(RwLock::new(persistent_state)),
            volatile_state: Arc::new(RwLock::new(VolatileState::default())),
            leader_state: Arc::new(RwLock::new(None)),
            log: Arc::new(RwLock::new(log)),
            storage,
            config,
            peers: Arc::new(RwLock::new(HashMap::new())),
            snapshot_manager: Arc::new(RwLock::new(snapshot_manager)),
            pre_vote_state: Arc::new(RwLock::new(PreVoteState::new())),
            shutdown_tx: None,
        })
    }
    
    /// Get the node ID
    pub fn id(&self) -> NodeId {
        self.id
    }
    
    /// Get current term
    pub async fn current_term(&self) -> Term {
        self.persistent_state.read().await.current_term
    }
    
    /// Get current role
    pub async fn role(&self) -> Role {
        self.volatile_state.read().await.role
    }
    
    /// Get current leader ID
    pub async fn leader_id(&self) -> Option<NodeId> {
        self.volatile_state.read().await.leader_id
    }
    
    /// Add a peer to the cluster
    pub async fn add_peer(&self, peer_id: NodeId, address: String) {
        self.peers.write().await.insert(peer_id, address);
    }
    
    /// Remove a peer from the cluster
    pub async fn remove_peer(&self, peer_id: NodeId) {
        self.peers.write().await.remove(&peer_id);
    }
    
    /// Start the RAFT node
    pub async fn start(&mut self) -> Result<()> {
        info!("Starting RAFT node {}", self.id);
        
        let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
        self.shutdown_tx = Some(shutdown_tx);
        
        // Clone necessary state for background tasks
        let id = self.id;
        let volatile_state = Arc::clone(&self.volatile_state);
        let config = self.config.clone();
        
        // Election timeout task
        tokio::spawn(async move {
            let mut election_timer = interval(Duration::from_millis(10));
            
            loop {
                tokio::select! {
                    _ = election_timer.tick() => {
                        let state = volatile_state.read().await;
                        let elapsed = state.last_heartbeat.elapsed();
                        
                        // Check if election timeout has expired
                        if elapsed >= state.election_timeout && state.role != Role::Leader {
                            drop(state);
                            // Trigger election
                            debug!("Node {} election timeout expired", id);
                            // TODO: Start election
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        info!("Shutting down election timer for node {}", id);
                        break;
                    }
                }
            }
        });
        
        Ok(())
    }
    
    /// Stop the RAFT node
    pub async fn stop(&mut self) -> Result<()> {
        info!("Stopping RAFT node {}", self.id);
        
        if let Some(tx) = self.shutdown_tx.take() {
            tx.send(()).await.ok();
        }
        
        Ok(())
    }
    
    /// Handle RequestVote RPC
    pub async fn handle_request_vote(&self, req: RequestVoteRequest) -> Result<RequestVoteResponse> {
        let mut persistent = self.persistent_state.write().await;
        let mut volatile = self.volatile_state.write().await;
        let log = self.log.read().await;
        
        debug!(
            "Node {} received vote request from {} for term {}",
            self.id, req.candidate_id, req.term
        );
        
        // Update term if request has higher term
        if req.term > persistent.current_term {
            persistent.current_term = req.term;
            persistent.voted_for = None;
            volatile.role = Role::Follower;
            volatile.leader_id = None;
            
            // Persist state
            self.storage.save_state(&persistent).await?;
        }
        
        // Reject if request has lower term
        if req.term < persistent.current_term {
            return Ok(RequestVoteResponse {
                term: persistent.current_term,
                vote_granted: false,
                pre_vote: req.pre_vote,
            });
        }
        
        // Check if we can grant vote
        let can_vote = persistent.voted_for.is_none() 
            || persistent.voted_for == Some(req.candidate_id);
        
        // Check if candidate's log is at least as up-to-date as ours
        let last_log_term = log.last_term();
        let last_log_index = log.last_index();
        
        let log_ok = req.last_log_term > last_log_term
            || (req.last_log_term == last_log_term && req.last_log_index >= last_log_index);
        
        let vote_granted = can_vote && log_ok;
        
        if vote_granted && !req.pre_vote {
            persistent.voted_for = Some(req.candidate_id);
            volatile.last_heartbeat = Instant::now();
            
            // Persist state
            self.storage.save_state(&persistent).await?;
            
            info!(
                "Node {} granted vote to {} for term {}",
                self.id, req.candidate_id, req.term
            );
        }
        
        Ok(RequestVoteResponse {
            term: persistent.current_term,
            vote_granted,
            pre_vote: req.pre_vote,
        })
    }
    
    /// Handle AppendEntries RPC
    pub async fn handle_append_entries(&self, req: AppendEntriesRequest) -> Result<AppendEntriesResponse> {
        let mut persistent = self.persistent_state.write().await;
        let mut volatile = self.volatile_state.write().await;
        let mut log = self.log.write().await;
        
        debug!(
            "Node {} received append entries from {} for term {}, entries: {}",
            self.id, req.leader_id, req.term, req.entries.len()
        );
        
        // Update term if request has higher term
        if req.term > persistent.current_term {
            persistent.current_term = req.term;
            persistent.voted_for = None;
            volatile.role = Role::Follower;
            
            // Persist state
            self.storage.save_state(&persistent).await?;
        }
        
        // Reject if request has lower term
        if req.term < persistent.current_term {
            return Ok(AppendEntriesResponse {
                term: persistent.current_term,
                success: false,
                match_index: 0,
                conflict_index: None,
                conflict_term: None,
            });
        }
        
        // Accept leader and reset election timeout
        volatile.leader_id = Some(req.leader_id);
        volatile.last_heartbeat = Instant::now();
        
        // Check log consistency
        if req.prev_log_index > 0 {
            match log.term(req.prev_log_index) {
                Some(term) if term == req.prev_log_term => {
                    // Log matches, proceed
                }
                Some(conflict_term) => {
                    // Log conflicts - find first index of conflicting term
                    let mut conflict_index = req.prev_log_index;
                    while conflict_index > 0 {
                        if log.term(conflict_index - 1) != Some(conflict_term) {
                            break;
                        }
                        conflict_index -= 1;
                    }
                    
                    return Ok(AppendEntriesResponse {
                        term: persistent.current_term,
                        success: false,
                        match_index: 0,
                        conflict_index: Some(conflict_index),
                        conflict_term: Some(conflict_term),
                    });
                }
                None => {
                    // Log is too short
                    return Ok(AppendEntriesResponse {
                        term: persistent.current_term,
                        success: false,
                        match_index: 0,
                        conflict_index: Some(log.last_index() + 1),
                        conflict_term: None,
                    });
                }
            }
        }
        
        // Append new entries
        if !req.entries.is_empty() {
            log.append(req.entries.clone());
            
            // Persist entries
            self.storage.append_entries(&req.entries).await?;
        }
        
        // Update commit index
        if req.leader_commit > log.commit_index() {
            let new_commit = req.leader_commit.min(log.last_index());
            log.set_commit_index(new_commit);
        }
        
        Ok(AppendEntriesResponse {
            term: persistent.current_term,
            success: true,
            match_index: log.last_index(),
            conflict_index: None,
            conflict_term: None,
        })
    }
    
    /// Handle InstallSnapshot RPC
    pub async fn handle_install_snapshot(
        &self,
        req: InstallSnapshotRequest,
    ) -> Result<InstallSnapshotResponse> {
        let mut persistent = self.persistent_state.write().await;
        let mut volatile = self.volatile_state.write().await;
        let mut log = self.log.write().await;
        
        info!(
            "Node {} received install snapshot from {} (index: {}, term: {})",
            self.id, req.leader_id, req.last_included_index, req.last_included_term
        );
        
        // Update term if request has higher term
        if req.term > persistent.current_term {
            persistent.current_term = req.term;
            persistent.voted_for = None;
            volatile.role = Role::Follower;
            
            // Persist state
            self.storage.save_state(&persistent).await?;
        }
        
        // Reject if request has lower term
        if req.term < persistent.current_term {
            return Ok(InstallSnapshotResponse {
                term: persistent.current_term,
                success: false,
            });
        }
        
        // Accept leader and reset election timeout
        volatile.leader_id = Some(req.leader_id);
        volatile.last_heartbeat = Instant::now();
        
        // Install snapshot
        log.install_snapshot(req.last_included_index, req.last_included_term);
        
        // Save snapshot metadata
        self.storage
            .save_snapshot_metadata(req.last_included_index, req.last_included_term)
            .await?;
        
        Ok(InstallSnapshotResponse {
            term: persistent.current_term,
            success: true,
        })
    }
    
    /// Propose a command to be replicated
    pub async fn propose(&self, command: Vec<u8>) -> Result<LogIndex> {
        // Check if we're the leader
        let volatile = self.volatile_state.read().await;
        if volatile.role != Role::Leader {
            return Err(RaftError::NotLeader(volatile.leader_id));
        }
        drop(volatile);
        
        let persistent = self.persistent_state.read().await;
        let term = persistent.current_term;
        drop(persistent);
        
        let mut log = self.log.write().await;
        let index = log.last_index() + 1;
        
        let entry = LogEntry::new_command(index, term, command);
        log.append(vec![entry.clone()]);
        
        // Persist entry
        self.storage.append_entries(&[entry]).await?;
        
        Ok(index)
    }
    
    /// Get the last committed index
    pub async fn commit_index(&self) -> LogIndex {
        self.log.read().await.commit_index()
    }
    
    /// Get committed entries that haven't been applied yet
    pub async fn committed_entries(&self) -> Vec<LogEntry> {
        let log = self.log.read().await;
        let last_applied = log.last_applied();
        let commit_index = log.commit_index();
        
        if commit_index <= last_applied {
            return Vec::new();
        }
        
        log.entries_range(last_applied + 1, commit_index + 1)
    }
    
    /// Mark entries as applied
    pub async fn mark_applied(&self, index: LogIndex) -> Result<()> {
        let mut log = self.log.write().await;
        log.set_last_applied(index);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::raft::MemoryStorage;
    use tempfile::TempDir;
    
    async fn create_test_node() -> RaftNode<MemoryStorage> {
        let storage = Arc::new(MemoryStorage::new());
        let config = RaftConfig::default();
        let temp_dir = TempDir::new().unwrap();
        
        RaftNode::new(1, storage, config, temp_dir.path().to_path_buf())
            .await
            .unwrap()
    }
    
    #[tokio::test]
    async fn test_node_creation() {
        let node = create_test_node().await;
        assert_eq!(node.id(), 1);
        assert_eq!(node.role().await, Role::Follower);
        assert_eq!(node.current_term().await, 0);
    }
    
    #[tokio::test]
    async fn test_handle_vote_request() {
        let node = create_test_node().await;
        
        let req = RequestVoteRequest::new(1, 2, 0, 0);
        let resp = node.handle_request_vote(req).await.unwrap();
        
        assert!(resp.vote_granted);
        assert_eq!(resp.term, 1);
    }
    
    #[tokio::test]
    async fn test_handle_append_entries() {
        let node = create_test_node().await;
        
        let req = AppendEntriesRequest::heartbeat(1, 2, 0, 0, 0);
        let resp = node.handle_append_entries(req).await.unwrap();
        
        assert!(resp.success);
        assert_eq!(resp.term, 1);
        assert_eq!(node.leader_id().await, Some(2));
    }
    
    #[tokio::test]
    async fn test_propose_command() {
        let node = create_test_node().await;
        
        // Become leader first (simplified for test)
        {
            let mut volatile = node.volatile_state.write().await;
            volatile.role = Role::Leader;
            volatile.leader_id = Some(node.id);
        }
        
        let command = b"test command".to_vec();
        let index = node.propose(command).await.unwrap();
        
        assert_eq!(index, 1);
    }
}

