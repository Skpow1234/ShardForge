//! RAFT RPC message definitions

use serde::{Deserialize, Serialize};

use super::{LogEntry, LogIndex, NodeId, Term};

/// RequestVote RPC - invoked by candidates to gather votes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteRequest {
    /// Candidate's term
    pub term: Term,
    
    /// Candidate requesting vote
    pub candidate_id: NodeId,
    
    /// Index of candidate's last log entry
    pub last_log_index: LogIndex,
    
    /// Term of candidate's last log entry
    pub last_log_term: Term,
    
    /// Whether this is a pre-vote (used in pre-vote protocol)
    pub pre_vote: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestVoteResponse {
    /// Current term, for candidate to update itself
    pub term: Term,
    
    /// True means candidate received vote
    pub vote_granted: bool,
    
    /// Whether this response is for a pre-vote
    pub pre_vote: bool,
}

/// AppendEntries RPC - used for log replication and heartbeat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesRequest {
    /// Leader's term
    pub term: Term,
    
    /// So follower can redirect clients
    pub leader_id: NodeId,
    
    /// Index of log entry immediately preceding new ones
    pub prev_log_index: LogIndex,
    
    /// Term of prev_log_index entry
    pub prev_log_term: Term,
    
    /// Log entries to store (empty for heartbeat)
    pub entries: Vec<LogEntry>,
    
    /// Leader's commit_index
    pub leader_commit: LogIndex,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppendEntriesResponse {
    /// Current term, for leader to update itself
    pub term: Term,
    
    /// True if follower contained entry matching prev_log_index and prev_log_term
    pub success: bool,
    
    /// Follower's last log index (for updating next_index)
    pub match_index: LogIndex,
    
    /// Hint for next_index on failure
    pub conflict_index: Option<LogIndex>,
    
    /// Term of conflicting entry
    pub conflict_term: Option<Term>,
}

/// InstallSnapshot RPC - used to send snapshot when follower is too far behind
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotRequest {
    /// Leader's term
    pub term: Term,
    
    /// Leader ID for follower to redirect clients
    pub leader_id: NodeId,
    
    /// The snapshot replaces all entries up through and including this index
    pub last_included_index: LogIndex,
    
    /// Term of last_included_index
    pub last_included_term: Term,
    
    /// Byte offset where chunk is positioned in the snapshot file
    pub offset: u64,
    
    /// Raw bytes of the snapshot chunk
    pub data: Vec<u8>,
    
    /// True if this is the last chunk
    pub done: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallSnapshotResponse {
    /// Current term, for leader to update itself
    pub term: Term,
    
    /// True if snapshot was successfully installed
    pub success: bool,
}

/// TimeoutNow RPC - used for leader transfer
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutNowRequest {
    /// Leader's term
    pub term: Term,
    
    /// Leader ID initiating the transfer
    pub leader_id: NodeId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeoutNowResponse {
    /// Current term
    pub term: Term,
    
    /// Whether the node accepted the timeout
    pub success: bool,
}

/// Configuration change request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigChangeRequest {
    /// Node to add or remove
    pub node_id: NodeId,
    
    /// Node address
    pub address: String,
    
    /// Whether this is an addition (true) or removal (false)
    pub add: bool,
}

/// RPC message envelope
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RpcMessage {
    RequestVote(RequestVoteRequest),
    RequestVoteResponse(RequestVoteResponse),
    AppendEntries(AppendEntriesRequest),
    AppendEntriesResponse(AppendEntriesResponse),
    InstallSnapshot(InstallSnapshotRequest),
    InstallSnapshotResponse(InstallSnapshotResponse),
    TimeoutNow(TimeoutNowRequest),
    TimeoutNowResponse(TimeoutNowResponse),
}

impl AppendEntriesRequest {
    /// Create a heartbeat (empty append entries)
    pub fn heartbeat(term: Term, leader_id: NodeId, prev_log_index: LogIndex, prev_log_term: Term, leader_commit: LogIndex) -> Self {
        Self {
            term,
            leader_id,
            prev_log_index,
            prev_log_term,
            entries: Vec::new(),
            leader_commit,
        }
    }
    
    /// Check if this is a heartbeat
    pub fn is_heartbeat(&self) -> bool {
        self.entries.is_empty()
    }
}

impl RequestVoteRequest {
    /// Create a vote request
    pub fn new(term: Term, candidate_id: NodeId, last_log_index: LogIndex, last_log_term: Term) -> Self {
        Self {
            term,
            candidate_id,
            last_log_index,
            last_log_term,
            pre_vote: false,
        }
    }
    
    /// Create a pre-vote request
    pub fn pre_vote(term: Term, candidate_id: NodeId, last_log_index: LogIndex, last_log_term: Term) -> Self {
        Self {
            term,
            candidate_id,
            last_log_index,
            last_log_term,
            pre_vote: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_heartbeat_creation() {
        let heartbeat = AppendEntriesRequest::heartbeat(1, 1, 10, 1, 8);
        
        assert!(heartbeat.is_heartbeat());
        assert_eq!(heartbeat.term, 1);
        assert_eq!(heartbeat.leader_id, 1);
        assert_eq!(heartbeat.entries.len(), 0);
    }
    
    #[test]
    fn test_vote_request() {
        let vote_req = RequestVoteRequest::new(2, 3, 10, 1);
        
        assert_eq!(vote_req.term, 2);
        assert_eq!(vote_req.candidate_id, 3);
        assert!(!vote_req.pre_vote);
    }
    
    #[test]
    fn test_pre_vote_request() {
        let pre_vote_req = RequestVoteRequest::pre_vote(2, 3, 10, 1);
        
        assert_eq!(pre_vote_req.term, 2);
        assert_eq!(pre_vote_req.candidate_id, 3);
        assert!(pre_vote_req.pre_vote);
    }
}

