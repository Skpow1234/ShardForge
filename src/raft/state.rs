//! RAFT node state management

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Instant;

use super::{LogIndex, NodeId, Role, Term};

/// Persistent state (must be saved to stable storage)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistentState {
    /// Latest term server has seen (initialized to 0)
    pub current_term: Term,
    
    /// Candidate ID that received vote in current term (or None)
    pub voted_for: Option<NodeId>,
    
    /// Cluster ID for identifying the cluster
    pub cluster_id: String,
}

impl Default for PersistentState {
    fn default() -> Self {
        Self {
            current_term: 0,
            voted_for: None,
            cluster_id: String::new(),
        }
    }
}

/// Volatile state (can be reconstructed on restart)
#[derive(Debug, Clone)]
pub struct VolatileState {
    /// Current role (Follower, Candidate, or Leader)
    pub role: Role,
    
    /// Current leader ID (if known)
    pub leader_id: Option<NodeId>,
    
    /// Time of last message from leader
    pub last_heartbeat: Instant,
    
    /// Election timeout duration
    pub election_timeout: std::time::Duration,
    
    /// Number of votes received in current election
    pub votes_received: usize,
    
    /// Set of nodes that voted for us in current election
    pub voters: HashMap<NodeId, bool>,
}

impl Default for VolatileState {
    fn default() -> Self {
        Self {
            role: Role::Follower,
            leader_id: None,
            last_heartbeat: Instant::now(),
            election_timeout: std::time::Duration::from_millis(150),
            votes_received: 0,
            voters: HashMap::new(),
        }
    }
}

/// Leader-specific volatile state
#[derive(Debug, Clone)]
pub struct LeaderState {
    /// For each server, index of next log entry to send
    pub next_index: HashMap<NodeId, LogIndex>,
    
    /// For each server, index of highest log entry known to be replicated
    pub match_index: HashMap<NodeId, LogIndex>,
    
    /// Last time each follower was contacted
    pub last_contact: HashMap<NodeId, Instant>,
    
    /// Number of in-flight replication requests per follower
    pub inflight_replications: HashMap<NodeId, usize>,
}

impl LeaderState {
    /// Create new leader state for given followers
    pub fn new(followers: &[NodeId], last_log_index: LogIndex) -> Self {
        let mut next_index = HashMap::new();
        let mut match_index = HashMap::new();
        let mut last_contact = HashMap::new();
        let mut inflight_replications = HashMap::new();
        
        for &follower in followers {
            next_index.insert(follower, last_log_index + 1);
            match_index.insert(follower, 0);
            last_contact.insert(follower, Instant::now());
            inflight_replications.insert(follower, 0);
        }
        
        Self {
            next_index,
            match_index,
            last_contact,
            inflight_replications,
        }
    }
    
    /// Update next_index for a follower (typically after failed append)
    pub fn decrement_next_index(&mut self, follower: NodeId) {
        if let Some(next_idx) = self.next_index.get_mut(&follower) {
            if *next_idx > 1 {
                *next_idx -= 1;
            }
        }
    }
    
    /// Update indices after successful replication
    pub fn update_progress(&mut self, follower: NodeId, match_idx: LogIndex) {
        self.match_index.insert(follower, match_idx);
        self.next_index.insert(follower, match_idx + 1);
        self.last_contact.insert(follower, Instant::now());
    }
    
    /// Check if a follower is lagging
    pub fn is_lagging(&self, follower: NodeId, leader_index: LogIndex, threshold: u64) -> bool {
        if let Some(&match_idx) = self.match_index.get(&follower) {
            leader_index - match_idx > threshold
        } else {
            false
        }
    }
    
    /// Get the median match_index (for commit index calculation)
    pub fn calculate_commit_index(&self, leader_match_index: LogIndex) -> LogIndex {
        let mut indices: Vec<LogIndex> = self.match_index.values().copied().collect();
        indices.push(leader_match_index); // Include leader's own index
        
        indices.sort_unstable();
        
        // Return the median (majority index)
        let majority_pos = indices.len() / 2;
        indices[majority_pos]
    }
}

/// Pre-vote state for preventing disruptive elections
#[derive(Debug, Clone)]
pub struct PreVoteState {
    /// Number of pre-votes received
    pub pre_votes_received: usize,
    
    /// Nodes that granted pre-vote
    pub pre_voters: HashMap<NodeId, bool>,
}

impl PreVoteState {
    pub fn new() -> Self {
        Self {
            pre_votes_received: 0,
            pre_voters: HashMap::new(),
        }
    }
    
    pub fn reset(&mut self) {
        self.pre_votes_received = 0;
        self.pre_voters.clear();
    }
}

impl Default for PreVoteState {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_leader_state_initialization() {
        let followers = vec![1, 2, 3];
        let leader_state = LeaderState::new(&followers, 10);
        
        assert_eq!(leader_state.next_index.len(), 3);
        assert_eq!(leader_state.match_index.len(), 3);
        
        for &follower in &followers {
            assert_eq!(*leader_state.next_index.get(&follower).unwrap(), 11);
            assert_eq!(*leader_state.match_index.get(&follower).unwrap(), 0);
        }
    }
    
    #[test]
    fn test_decrement_next_index() {
        let mut leader_state = LeaderState::new(&[1, 2], 10);
        
        leader_state.decrement_next_index(1);
        assert_eq!(*leader_state.next_index.get(&1).unwrap(), 10);
        
        // Should not go below 1
        for _ in 0..20 {
            leader_state.decrement_next_index(1);
        }
        assert_eq!(*leader_state.next_index.get(&1).unwrap(), 1);
    }
    
    #[test]
    fn test_update_progress() {
        let mut leader_state = LeaderState::new(&[1, 2], 10);
        
        leader_state.update_progress(1, 8);
        
        assert_eq!(*leader_state.match_index.get(&1).unwrap(), 8);
        assert_eq!(*leader_state.next_index.get(&1).unwrap(), 9);
    }
    
    #[test]
    fn test_calculate_commit_index() {
        let mut leader_state = LeaderState::new(&[1, 2, 3], 10);
        
        leader_state.match_index.insert(1, 5);
        leader_state.match_index.insert(2, 7);
        leader_state.match_index.insert(3, 6);
        
        // With leader at 10, median should be 7
        let commit_index = leader_state.calculate_commit_index(10);
        assert_eq!(commit_index, 7);
    }
    
    #[test]
    fn test_is_lagging() {
        let mut leader_state = LeaderState::new(&[1, 2], 10);
        
        leader_state.match_index.insert(1, 5);
        
        assert!(!leader_state.is_lagging(1, 10, 10));
        assert!(leader_state.is_lagging(1, 10, 4));
    }
}

