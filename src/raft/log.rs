//! RAFT log implementation

use serde::{Deserialize, Serialize};

use super::{LogIndex, Term};

/// Log entry in the RAFT log
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LogEntry {
    /// Log index (1-based)
    pub index: LogIndex,
    
    /// Term when entry was received by leader
    pub term: Term,
    
    /// Command to apply to state machine
    pub command: Vec<u8>,
    
    /// Entry type
    pub entry_type: LogEntryType,
}

/// Type of log entry
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LogEntryType {
    /// Normal command entry
    Command,
    
    /// Configuration change entry
    Configuration,
    
    /// No-op entry (inserted by new leader)
    NoOp,
}

impl LogEntry {
    /// Create a new command entry
    pub fn new_command(index: LogIndex, term: Term, command: Vec<u8>) -> Self {
        Self {
            index,
            term,
            command,
            entry_type: LogEntryType::Command,
        }
    }
    
    /// Create a new no-op entry
    pub fn new_noop(index: LogIndex, term: Term) -> Self {
        Self {
            index,
            term,
            command: Vec::new(),
            entry_type: LogEntryType::NoOp,
        }
    }
    
    /// Create a new configuration entry
    pub fn new_config(index: LogIndex, term: Term, config: Vec<u8>) -> Self {
        Self {
            index,
            term,
            command: config,
            entry_type: LogEntryType::Configuration,
        }
    }
}

/// In-memory log with disk persistence
#[derive(Debug, Clone)]
pub struct RaftLog {
    /// Log entries (indexed from 1)
    entries: Vec<LogEntry>,
    
    /// Index of highest log entry known to be committed
    commit_index: LogIndex,
    
    /// Index of highest log entry applied to state machine
    last_applied: LogIndex,
    
    /// Last included index from snapshot
    snapshot_index: LogIndex,
    
    /// Last included term from snapshot
    snapshot_term: Term,
}

impl RaftLog {
    /// Create a new empty log
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            commit_index: 0,
            last_applied: 0,
            snapshot_index: 0,
            snapshot_term: 0,
        }
    }
    
    /// Get the last log index
    pub fn last_index(&self) -> LogIndex {
        if let Some(entry) = self.entries.last() {
            entry.index
        } else {
            self.snapshot_index
        }
    }
    
    /// Get the last log term
    pub fn last_term(&self) -> Term {
        if let Some(entry) = self.entries.last() {
            entry.term
        } else {
            self.snapshot_term
        }
    }
    
    /// Get log entry at index
    pub fn get(&self, index: LogIndex) -> Option<&LogEntry> {
        if index <= self.snapshot_index {
            None
        } else {
            let offset = (index - self.snapshot_index - 1) as usize;
            self.entries.get(offset)
        }
    }
    
    /// Get term of log entry at index
    pub fn term(&self, index: LogIndex) -> Option<Term> {
        if index == self.snapshot_index {
            Some(self.snapshot_term)
        } else {
            self.get(index).map(|entry| entry.term)
        }
    }
    
    /// Append new entries to log
    pub fn append(&mut self, entries: Vec<LogEntry>) {
        for entry in entries {
            // Remove any conflicting entries
            if entry.index <= self.last_index() {
                let offset = (entry.index - self.snapshot_index - 1) as usize;
                self.entries.truncate(offset);
            }
            
            self.entries.push(entry);
        }
    }
    
    /// Get entries starting from index
    pub fn entries_from(&self, start_index: LogIndex) -> Vec<LogEntry> {
        if start_index <= self.snapshot_index {
            return Vec::new();
        }
        
        let offset = (start_index - self.snapshot_index - 1) as usize;
        self.entries[offset..].to_vec()
    }
    
    /// Get entries in range [start, end)
    pub fn entries_range(&self, start: LogIndex, end: LogIndex) -> Vec<LogEntry> {
        if start <= self.snapshot_index {
            return Vec::new();
        }
        
        let start_offset = (start - self.snapshot_index - 1) as usize;
        let end_offset = (end - self.snapshot_index - 1) as usize;
        
        let end_offset = end_offset.min(self.entries.len());
        
        if start_offset >= self.entries.len() {
            return Vec::new();
        }
        
        self.entries[start_offset..end_offset].to_vec()
    }
    
    /// Truncate log after index
    pub fn truncate(&mut self, after_index: LogIndex) {
        if after_index <= self.snapshot_index {
            self.entries.clear();
        } else {
            let offset = (after_index - self.snapshot_index) as usize;
            self.entries.truncate(offset);
        }
    }
    
    /// Get commit index
    pub fn commit_index(&self) -> LogIndex {
        self.commit_index
    }
    
    /// Set commit index
    pub fn set_commit_index(&mut self, index: LogIndex) {
        self.commit_index = index;
    }
    
    /// Get last applied index
    pub fn last_applied(&self) -> LogIndex {
        self.last_applied
    }
    
    /// Set last applied index
    pub fn set_last_applied(&mut self, index: LogIndex) {
        self.last_applied = index;
    }
    
    /// Install snapshot
    pub fn install_snapshot(&mut self, last_included_index: LogIndex, last_included_term: Term) {
        self.snapshot_index = last_included_index;
        self.snapshot_term = last_included_term;
        
        // Remove log entries included in snapshot
        if last_included_index >= self.last_index() {
            self.entries.clear();
        } else {
            let offset = (last_included_index - self.snapshot_index) as usize;
            self.entries.drain(..offset);
        }
        
        // Update commit and applied indices
        if self.commit_index < last_included_index {
            self.commit_index = last_included_index;
        }
        if self.last_applied < last_included_index {
            self.last_applied = last_included_index;
        }
    }
    
    /// Get snapshot index
    pub fn snapshot_index(&self) -> LogIndex {
        self.snapshot_index
    }
    
    /// Get snapshot term
    pub fn snapshot_term(&self) -> Term {
        self.snapshot_term
    }
    
    /// Get number of entries in log
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    
    /// Check if log is empty
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

impl Default for RaftLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_empty_log() {
        let log = RaftLog::new();
        assert_eq!(log.last_index(), 0);
        assert_eq!(log.last_term(), 0);
        assert!(log.is_empty());
    }
    
    #[test]
    fn test_append_entries() {
        let mut log = RaftLog::new();
        
        let entries = vec![
            LogEntry::new_command(1, 1, b"command1".to_vec()),
            LogEntry::new_command(2, 1, b"command2".to_vec()),
        ];
        
        log.append(entries);
        
        assert_eq!(log.last_index(), 2);
        assert_eq!(log.last_term(), 1);
        assert_eq!(log.len(), 2);
    }
    
    #[test]
    fn test_get_entry() {
        let mut log = RaftLog::new();
        log.append(vec![LogEntry::new_command(1, 1, b"cmd".to_vec())]);
        
        assert!(log.get(1).is_some());
        assert!(log.get(2).is_none());
    }
    
    #[test]
    fn test_entries_range() {
        let mut log = RaftLog::new();
        log.append(vec![
            LogEntry::new_command(1, 1, b"c1".to_vec()),
            LogEntry::new_command(2, 1, b"c2".to_vec()),
            LogEntry::new_command(3, 2, b"c3".to_vec()),
        ]);
        
        let entries = log.entries_range(2, 4);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].index, 2);
        assert_eq!(entries[1].index, 3);
    }
    
    #[test]
    fn test_truncate() {
        let mut log = RaftLog::new();
        log.append(vec![
            LogEntry::new_command(1, 1, b"c1".to_vec()),
            LogEntry::new_command(2, 1, b"c2".to_vec()),
            LogEntry::new_command(3, 2, b"c3".to_vec()),
        ]);
        
        log.truncate(2);
        assert_eq!(log.last_index(), 2);
        assert_eq!(log.len(), 2);
    }
    
    #[test]
    fn test_commit_index() {
        let mut log = RaftLog::new();
        log.set_commit_index(5);
        assert_eq!(log.commit_index(), 5);
    }
    
    #[test]
    fn test_snapshot() {
        let mut log = RaftLog::new();
        log.append(vec![
            LogEntry::new_command(1, 1, b"c1".to_vec()),
            LogEntry::new_command(2, 1, b"c2".to_vec()),
            LogEntry::new_command(3, 2, b"c3".to_vec()),
        ]);
        
        log.install_snapshot(2, 1);
        
        assert_eq!(log.snapshot_index(), 2);
        assert_eq!(log.snapshot_term(), 1);
        assert_eq!(log.len(), 1); // Only entry 3 remains
    }
}

