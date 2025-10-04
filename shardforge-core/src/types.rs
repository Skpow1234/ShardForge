//! Core type definitions for ShardForge

use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

/// Unique identifier for nodes in the cluster
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct NodeId(Uuid);

impl NodeId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for NodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for NodeId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for databases
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct DatabaseId(Uuid);

impl DatabaseId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for DatabaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for DatabaseId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for tables
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TableId(Uuid);

impl TableId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for TableId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TableId {
    fn default() -> Self {
        Self::new()
    }
}

/// Unique identifier for transactions
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TransactionId(Uuid);

impl TransactionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_string(s: &str) -> Result<Self, uuid::Error> {
        Ok(Self(Uuid::parse_str(s)?))
    }

    pub fn to_string(&self) -> String {
        self.0.to_string()
    }
}

impl fmt::Display for TransactionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Default for TransactionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Key for data storage operations
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Key(Vec<u8>);

impl Key {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn from_string(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Value for data storage operations
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Value(Vec<u8>);

impl Value {
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }

    pub fn from_string(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }

    pub fn from_slice(slice: &[u8]) -> Self {
        Self(slice.to_vec())
    }

    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    pub fn into_vec(self) -> Vec<u8> {
        self.0
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self.0)
    }
}

/// Version identifier for MVCC
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Version(u64);

impl Version {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn increment(&self) -> Self {
        Self(self.0 + 1)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl Default for Version {
    fn default() -> Self {
        Self(0)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Timestamp for versioning and ordering
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct Timestamp(u64);

impl Timestamp {
    pub fn new(value: u64) -> Self {
        Self(value)
    }

    pub fn now() -> Self {
        use std::time::{SystemTime, UNIX_EPOCH};
        let duration = SystemTime::now().duration_since(UNIX_EPOCH).expect("Time went backwards");
        Self(duration.as_micros() as u64)
    }

    pub fn value(&self) -> u64 {
        self.0
    }
}

impl fmt::Display for Timestamp {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}
