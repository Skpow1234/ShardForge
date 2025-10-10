//! Index implementations for ShardForge

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use shardforge_core::{Key, Result, ShardForgeError};
use tokio::sync::RwLock;

/// Index trait for different index types
#[async_trait::async_trait]
pub trait Index: Send + Sync {
    /// Insert a key-value pair into the index
    async fn insert(&self, key: IndexKey, value: IndexValue) -> Result<()>;
    
    /// Remove a key from the index
    async fn remove(&self, key: &IndexKey) -> Result<Option<IndexValue>>;
    
    /// Get a value by key
    async fn get(&self, key: &IndexKey) -> Result<Option<IndexValue>>;
    
    /// Get all values within a range (for range queries)
    async fn range(&self, start: &IndexKey, end: &IndexKey) -> Result<Vec<(IndexKey, IndexValue)>>;
    
    /// Get all key-value pairs (for full scans)
    async fn scan(&self) -> Result<Vec<(IndexKey, IndexValue)>>;
    
    /// Get index statistics
    async fn stats(&self) -> IndexStats;
    
    /// Clear all entries from the index
    async fn clear(&self) -> Result<()>;
}

/// Index key type
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct IndexKey(Vec<u8>);

/// Index value type (typically points to the actual data)
#[derive(Debug, Clone, PartialEq)]
pub struct IndexValue(Vec<u8>);

/// Index statistics
#[derive(Debug, Clone, Default)]
pub struct IndexStats {
    /// Number of entries in the index
    pub entry_count: usize,
    /// Size of the index in bytes (approximate)
    pub size_bytes: usize,
    /// Number of lookups performed
    pub lookup_count: u64,
    /// Number of insertions performed
    pub insert_count: u64,
    /// Number of deletions performed
    pub delete_count: u64,
}

/// B-tree index implementation
pub struct BTreeIndex {
    /// The underlying B-tree map
    data: Arc<RwLock<BTreeMap<IndexKey, IndexValue>>>,
    /// Index statistics
    stats: Arc<RwLock<IndexStats>>,
}

/// Hash index implementation
pub struct HashIndex {
    /// The underlying hash map
    data: Arc<RwLock<HashMap<IndexKey, IndexValue>>>,
    /// Index statistics
    stats: Arc<RwLock<IndexStats>>,
}

/// Index factory for creating different types of indexes
pub struct IndexFactory;

/// Index type enumeration
#[derive(Debug, Clone, PartialEq)]
pub enum IndexType {
    BTree,
    Hash,
}

impl IndexKey {
    /// Create a new index key
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }
    
    /// Create an index key from a string
    pub fn from_string(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
    
    /// Create an index key from an integer
    pub fn from_i64(value: i64) -> Self {
        Self(value.to_be_bytes().to_vec())
    }
    
    /// Create an index key from a float
    pub fn from_f64(value: f64) -> Self {
        Self(value.to_be_bytes().to_vec())
    }
    
    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Convert to a string (if possible)
    pub fn to_string(&self) -> Result<String> {
        String::from_utf8(self.0.clone()).map_err(|_| ShardForgeError::Index {
            message: "Index key is not valid UTF-8".to_string(),
        })
    }
    
    /// Convert to an integer (if possible)
    pub fn to_i64(&self) -> Result<i64> {
        if self.0.len() == 8 {
            let mut bytes = [0u8; 8];
            bytes.copy_from_slice(&self.0);
            Ok(i64::from_be_bytes(bytes))
        } else {
            Err(ShardForgeError::Index {
                message: "Index key is not a valid i64".to_string(),
            })
        }
    }
}

impl IndexValue {
    /// Create a new index value
    pub fn new(data: Vec<u8>) -> Self {
        Self(data)
    }
    
    /// Create an index value from a primary key
    pub fn from_primary_key(key: &Key) -> Self {
        Self(key.as_ref().to_vec())
    }
    
    /// Create an index value from a string
    pub fn from_string(s: &str) -> Self {
        Self(s.as_bytes().to_vec())
    }
    
    /// Get the raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
    
    /// Convert to a primary key
    pub fn to_primary_key(&self) -> Key {
        Key::new(&self.0)
    }
    
    /// Convert to a string (if possible)
    pub fn to_string(&self) -> Result<String> {
        String::from_utf8(self.0.clone()).map_err(|_| ShardForgeError::Index {
            message: "Index value is not valid UTF-8".to_string(),
        })
    }
}

impl BTreeIndex {
    /// Create a new B-tree index
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(BTreeMap::new())),
            stats: Arc::new(RwLock::new(IndexStats::default())),
        }
    }
}

#[async_trait::async_trait]
impl Index for BTreeIndex {
    async fn insert(&self, key: IndexKey, value: IndexValue) -> Result<()> {
        let mut data = self.data.write().await;
        let mut stats = self.stats.write().await;
        
        data.insert(key, value);
        stats.insert_count += 1;
        stats.entry_count = data.len();
        
        Ok(())
    }
    
    async fn remove(&self, key: &IndexKey) -> Result<Option<IndexValue>> {
        let mut data = self.data.write().await;
        let mut stats = self.stats.write().await;
        
        let result = data.remove(key);
        if result.is_some() {
            stats.delete_count += 1;
            stats.entry_count = data.len();
        }
        
        Ok(result)
    }
    
    async fn get(&self, key: &IndexKey) -> Result<Option<IndexValue>> {
        let data = self.data.read().await;
        let mut stats = self.stats.write().await;
        
        stats.lookup_count += 1;
        Ok(data.get(key).cloned())
    }
    
    async fn range(&self, start: &IndexKey, end: &IndexKey) -> Result<Vec<(IndexKey, IndexValue)>> {
        let data = self.data.read().await;
        let mut stats = self.stats.write().await;
        
        stats.lookup_count += 1;
        
        let result = data
            .range(start..=end)
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        Ok(result)
    }
    
    async fn scan(&self) -> Result<Vec<(IndexKey, IndexValue)>> {
        let data = self.data.read().await;
        let mut stats = self.stats.write().await;
        
        stats.lookup_count += 1;
        
        let result = data
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        Ok(result)
    }
    
    async fn stats(&self) -> IndexStats {
        let stats = self.stats.read().await;
        let data = self.data.read().await;
        
        let mut result = stats.clone();
        result.entry_count = data.len();
        
        // Rough estimate of size
        result.size_bytes = data.len() * 64; // Approximate size per entry
        
        result
    }
    
    async fn clear(&self) -> Result<()> {
        let mut data = self.data.write().await;
        let mut stats = self.stats.write().await;
        
        data.clear();
        stats.entry_count = 0;
        
        Ok(())
    }
}

impl HashIndex {
    /// Create a new hash index
    pub fn new() -> Self {
        Self {
            data: Arc::new(RwLock::new(HashMap::new())),
            stats: Arc::new(RwLock::new(IndexStats::default())),
        }
    }
}

#[async_trait::async_trait]
impl Index for HashIndex {
    async fn insert(&self, key: IndexKey, value: IndexValue) -> Result<()> {
        let mut data = self.data.write().await;
        let mut stats = self.stats.write().await;
        
        data.insert(key, value);
        stats.insert_count += 1;
        stats.entry_count = data.len();
        
        Ok(())
    }
    
    async fn remove(&self, key: &IndexKey) -> Result<Option<IndexValue>> {
        let mut data = self.data.write().await;
        let mut stats = self.stats.write().await;
        
        let result = data.remove(key);
        if result.is_some() {
            stats.delete_count += 1;
            stats.entry_count = data.len();
        }
        
        Ok(result)
    }
    
    async fn get(&self, key: &IndexKey) -> Result<Option<IndexValue>> {
        let data = self.data.read().await;
        let mut stats = self.stats.write().await;
        
        stats.lookup_count += 1;
        Ok(data.get(key).cloned())
    }
    
    async fn range(&self, _start: &IndexKey, _end: &IndexKey) -> Result<Vec<(IndexKey, IndexValue)>> {
        // Hash indexes don't support efficient range queries
        Err(ShardForgeError::Index {
            message: "Hash indexes do not support range queries".to_string(),
        })
    }
    
    async fn scan(&self) -> Result<Vec<(IndexKey, IndexValue)>> {
        let data = self.data.read().await;
        let mut stats = self.stats.write().await;
        
        stats.lookup_count += 1;
        
        let result = data
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        
        Ok(result)
    }
    
    async fn stats(&self) -> IndexStats {
        let stats = self.stats.read().await;
        let data = self.data.read().await;
        
        let mut result = stats.clone();
        result.entry_count = data.len();
        
        // Rough estimate of size
        result.size_bytes = data.len() * 64; // Approximate size per entry
        
        result
    }
    
    async fn clear(&self) -> Result<()> {
        let mut data = self.data.write().await;
        let mut stats = self.stats.write().await;
        
        data.clear();
        stats.entry_count = 0;
        
        Ok(())
    }
}

impl IndexFactory {
    /// Create a new index of the specified type
    pub fn create_index(index_type: IndexType) -> Box<dyn Index> {
        match index_type {
            IndexType::BTree => Box::new(BTreeIndex::new()),
            IndexType::Hash => Box::new(HashIndex::new()),
        }
    }
}

impl Default for BTreeIndex {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for HashIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Index manager for managing multiple indexes
pub struct IndexManager {
    /// Indexes by name
    indexes: Arc<RwLock<HashMap<String, Box<dyn Index>>>>,
}

impl IndexManager {
    /// Create a new index manager
    pub fn new() -> Self {
        Self {
            indexes: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Create and register a new index
    pub async fn create_index(&self, name: String, index_type: IndexType) -> Result<()> {
        let mut indexes = self.indexes.write().await;
        
        if indexes.contains_key(&name) {
            return Err(ShardForgeError::Index {
                message: format!("Index '{}' already exists", name),
            });
        }
        
        let index = IndexFactory::create_index(index_type);
        indexes.insert(name, index);
        
        Ok(())
    }
    
    /// Get an index by name
    pub async fn get_index(&self, _name: &str) -> Option<Arc<dyn Index + Send + Sync>> {
        let _indexes = self.indexes.read().await;
        // This is a simplified version - in a real implementation, we'd need to handle
        // the conversion from Box<dyn Index> to Arc<dyn Index + Send + Sync> properly
        None // Placeholder
    }
    
    /// Drop an index
    pub async fn drop_index(&self, name: &str) -> Result<()> {
        let mut indexes = self.indexes.write().await;
        
        if indexes.remove(name).is_some() {
            Ok(())
        } else {
            Err(ShardForgeError::Index {
                message: format!("Index '{}' does not exist", name),
            })
        }
    }
    
    /// List all index names
    pub async fn list_indexes(&self) -> Vec<String> {
        let indexes = self.indexes.read().await;
        indexes.keys().cloned().collect()
    }
    
    /// Get statistics for all indexes
    pub async fn get_all_stats(&self) -> HashMap<String, IndexStats> {
        let indexes = self.indexes.read().await;
        let mut stats = HashMap::new();
        
        for (name, index) in indexes.iter() {
            stats.insert(name.clone(), index.stats().await);
        }
        
        stats
    }
}

impl Default for IndexManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_btree_index_basic_operations() {
        let index = BTreeIndex::new();
        
        let key1 = IndexKey::from_string("key1");
        let value1 = IndexValue::from_string("value1");
        let key2 = IndexKey::from_string("key2");
        let value2 = IndexValue::from_string("value2");
        
        // Insert
        index.insert(key1.clone(), value1.clone()).await.unwrap();
        index.insert(key2.clone(), value2.clone()).await.unwrap();
        
        // Get
        let result = index.get(&key1).await.unwrap();
        assert_eq!(result, Some(value1.clone()));
        
        // Scan
        let all_entries = index.scan().await.unwrap();
        assert_eq!(all_entries.len(), 2);
        
        // Remove
        let removed = index.remove(&key1).await.unwrap();
        assert_eq!(removed, Some(value1));
        
        let result = index.get(&key1).await.unwrap();
        assert!(result.is_none());
        
        // Stats
        let stats = index.stats().await;
        assert_eq!(stats.entry_count, 1);
        assert!(stats.insert_count > 0);
        assert!(stats.lookup_count > 0);
    }

    #[tokio::test]
    async fn test_hash_index_basic_operations() {
        let index = HashIndex::new();
        
        let key1 = IndexKey::from_string("key1");
        let value1 = IndexValue::from_string("value1");
        
        // Insert
        index.insert(key1.clone(), value1.clone()).await.unwrap();
        
        // Get
        let result = index.get(&key1).await.unwrap();
        assert_eq!(result, Some(value1.clone()));
        
        // Range query should fail
        let key2 = IndexKey::from_string("key2");
        let range_result = index.range(&key1, &key2).await;
        assert!(range_result.is_err());
        
        // Remove
        let removed = index.remove(&key1).await.unwrap();
        assert_eq!(removed, Some(value1));
    }

    #[tokio::test]
    async fn test_btree_range_queries() {
        let index = BTreeIndex::new();
        
        // Insert some data
        for i in 1..=10 {
            let key = IndexKey::from_i64(i);
            let value = IndexValue::from_string(&format!("value{}", i));
            index.insert(key, value).await.unwrap();
        }
        
        // Range query
        let start = IndexKey::from_i64(3);
        let end = IndexKey::from_i64(7);
        let range_result = index.range(&start, &end).await.unwrap();
        
        assert_eq!(range_result.len(), 5); // 3, 4, 5, 6, 7
        
        // Verify the results are in order
        for (i, (key, _)) in range_result.iter().enumerate() {
            let expected_value = (i as i64) + 3;
            assert_eq!(key.to_i64().unwrap(), expected_value);
        }
    }

    #[tokio::test]
    async fn test_index_manager() {
        let manager = IndexManager::new();
        
        // Create indexes
        manager.create_index("btree_idx".to_string(), IndexType::BTree).await.unwrap();
        manager.create_index("hash_idx".to_string(), IndexType::Hash).await.unwrap();
        
        // List indexes
        let indexes = manager.list_indexes().await;
        assert_eq!(indexes.len(), 2);
        assert!(indexes.contains(&"btree_idx".to_string()));
        assert!(indexes.contains(&"hash_idx".to_string()));
        
        // Drop index
        manager.drop_index("btree_idx").await.unwrap();
        let indexes = manager.list_indexes().await;
        assert_eq!(indexes.len(), 1);
        
        // Try to drop non-existent index
        let result = manager.drop_index("nonexistent").await;
        assert!(result.is_err());
    }

    #[test]
    fn test_index_key_conversions() {
        // String conversion
        let key = IndexKey::from_string("hello");
        assert_eq!(key.to_string().unwrap(), "hello");
        
        // Integer conversion
        let key = IndexKey::from_i64(42);
        assert_eq!(key.to_i64().unwrap(), 42);
        
        // Float conversion
        let key = IndexKey::from_f64(3.14);
        assert_eq!(key.as_bytes().len(), 8);
    }

    #[test]
    fn test_index_value_conversions() {
        // String conversion
        let value = IndexValue::from_string("world");
        assert_eq!(value.to_string().unwrap(), "world");
        
        // Primary key conversion
        let primary_key = Key::new(b"pk123");
        let value = IndexValue::from_primary_key(&primary_key);
        let recovered_key = value.to_primary_key();
        assert_eq!(recovered_key.as_ref(), b"pk123");
    }
}
