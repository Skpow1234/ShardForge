//! Query result caching

use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use std::collections::hash_map::DefaultHasher;
use std::time::{Duration, SystemTime};

use shardforge_core::Result;

use crate::sql::executor::QueryResult;

/// Cache key for query results
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CacheKey {
    /// Hash of the SQL query
    query_hash: u64,
    /// Hash of query parameters
    params_hash: u64,
}

impl CacheKey {
    /// Create a new cache key from SQL and parameters
    pub fn new(sql: &str, params: &[u8]) -> Self {
        let mut hasher = DefaultHasher::new();
        sql.hash(&mut hasher);
        let query_hash = hasher.finish();

        let mut hasher = DefaultHasher::new();
        params.hash(&mut hasher);
        let params_hash = hasher.finish();

        Self {
            query_hash,
            params_hash,
        }
    }
}

/// Cached query result entry
#[derive(Debug, Clone)]
struct CacheEntry {
    /// The cached result
    result: QueryResult,
    /// When this entry was created
    created_at: SystemTime,
    /// When this entry was last accessed
    last_accessed: SystemTime,
    /// Number of times this entry was accessed
    access_count: u64,
    /// Size of the cached result in bytes
    size_bytes: usize,
}

/// Query result cache configuration
#[derive(Debug, Clone)]
pub struct CacheConfig {
    /// Maximum number of entries in the cache
    pub max_entries: usize,
    /// Maximum total size in bytes
    pub max_size_bytes: usize,
    /// Time-to-live for cache entries
    pub ttl: Duration,
    /// Enable LRU eviction
    pub enable_lru: bool,
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            max_size_bytes: 100 * 1024 * 1024, // 100 MB
            ttl: Duration::from_secs(300),      // 5 minutes
            enable_lru: true,
        }
    }
}

/// Query result cache
pub struct QueryCache {
    /// Cache entries
    entries: HashMap<CacheKey, CacheEntry>,
    /// Cache configuration
    config: CacheConfig,
    /// Total size of cached data
    total_size: usize,
    /// Cache statistics
    stats: CacheStatistics,
}

/// Cache statistics
#[derive(Debug, Clone, Default)]
pub struct CacheStatistics {
    /// Number of cache hits
    pub hits: u64,
    /// Number of cache misses
    pub misses: u64,
    /// Number of evictions
    pub evictions: u64,
    /// Number of insertions
    pub insertions: u64,
}

impl QueryCache {
    /// Create a new query cache with default configuration
    pub fn new() -> Self {
        Self::with_config(CacheConfig::default())
    }

    /// Create a new query cache with custom configuration
    pub fn with_config(config: CacheConfig) -> Self {
        Self {
            entries: HashMap::new(),
            config,
            total_size: 0,
            stats: CacheStatistics::default(),
        }
    }

    /// Get a cached query result
    pub fn get(&mut self, key: &CacheKey) -> Option<QueryResult> {
        if let Some(entry) = self.entries.get_mut(key) {
            // Check if entry has expired
            if let Ok(elapsed) = entry.created_at.elapsed() {
                if elapsed > self.config.ttl {
                    // Entry has expired, remove it
                    self.entries.remove(key);
                    self.stats.misses += 1;
                    return None;
                }
            }

            // Update access information
            entry.last_accessed = SystemTime::now();
            entry.access_count += 1;

            // Cache hit
            self.stats.hits += 1;
            Some(entry.result.clone())
        } else {
            // Cache miss
            self.stats.misses += 1;
            None
        }
    }

    /// Put a query result in the cache
    pub fn put(&mut self, key: CacheKey, result: QueryResult) -> Result<()> {
        // Estimate size of the result
        let size = self.estimate_result_size(&result);

        // Check if we need to evict entries
        while self.should_evict(size) {
            self.evict_one()?;
        }

        // Create cache entry
        let entry = CacheEntry {
            result,
            created_at: SystemTime::now(),
            last_accessed: SystemTime::now(),
            access_count: 0,
            size_bytes: size,
        };

        // Insert into cache
        if let Some(old_entry) = self.entries.insert(key, entry) {
            self.total_size -= old_entry.size_bytes;
        }
        self.total_size += size;
        self.stats.insertions += 1;

        Ok(())
    }

    /// Remove a specific entry from the cache
    pub fn remove(&mut self, key: &CacheKey) -> bool {
        if let Some(entry) = self.entries.remove(key) {
            self.total_size -= entry.size_bytes;
            true
        } else {
            false
        }
    }

    /// Clear all entries from the cache
    pub fn clear(&mut self) {
        self.entries.clear();
        self.total_size = 0;
    }

    /// Get cache statistics
    pub fn statistics(&self) -> &CacheStatistics {
        &self.stats
    }

    /// Get cache hit rate
    pub fn hit_rate(&self) -> f64 {
        let total_requests = self.stats.hits + self.stats.misses;
        if total_requests == 0 {
            0.0
        } else {
            self.stats.hits as f64 / total_requests as f64
        }
    }

    /// Get number of entries in cache
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }

    /// Get total size of cached data
    pub fn total_size(&self) -> usize {
        self.total_size
    }

    /// Check if we should evict an entry
    fn should_evict(&self, new_entry_size: usize) -> bool {
        // Check entry count limit
        if self.entries.len() >= self.config.max_entries {
            return true;
        }

        // Check size limit
        if self.total_size + new_entry_size > self.config.max_size_bytes {
            return true;
        }

        false
    }

    /// Evict one entry from the cache
    fn evict_one(&mut self) -> Result<()> {
        if self.entries.is_empty() {
            return Ok(());
        }

        // Find entry to evict based on strategy
        let key_to_evict = if self.config.enable_lru {
            // Find least recently used entry
            self.entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_accessed)
                .map(|(key, _)| key.clone())
        } else {
            // Find oldest entry
            self.entries
                .iter()
                .min_by_key(|(_, entry)| entry.created_at)
                .map(|(key, _)| key.clone())
        };

        if let Some(key) = key_to_evict {
            if let Some(entry) = self.entries.remove(&key) {
                self.total_size -= entry.size_bytes;
                self.stats.evictions += 1;
            }
        }

        Ok(())
    }

    /// Estimate the size of a query result
    fn estimate_result_size(&self, result: &QueryResult) -> usize {
        // Rough estimate: sum of column name sizes + row data sizes
        let columns_size: usize = result.columns.iter().map(|s| s.len()).sum();
        let rows_size: usize = result
            .rows
            .iter()
            .map(|row| row.iter().map(|v| v.as_ref().len()).sum::<usize>())
            .sum();

        columns_size + rows_size + 100 // Add some overhead
    }
}

impl Default for QueryCache {
    fn default() -> Self {
        Self::new()
    }
}

impl CacheStatistics {
    /// Calculate hit rate
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shardforge_core::Value;

    #[test]
    fn test_cache_basic_operations() {
        let mut cache = QueryCache::new();
        
        let key = CacheKey::new("SELECT * FROM users", b"");
        let result = QueryResult {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![vec![Value::new(b"1"), Value::new(b"Alice")]],
            affected_rows: 1,
        };

        // Put and get
        cache.put(key.clone(), result.clone()).unwrap();
        let cached = cache.get(&key);
        assert!(cached.is_some());

        // Statistics
        assert_eq!(cache.statistics().hits, 1);
        assert_eq!(cache.statistics().insertions, 1);
    }

    #[test]
    fn test_cache_miss() {
        let mut cache = QueryCache::new();
        
        let key = CacheKey::new("SELECT * FROM users", b"");
        let result = cache.get(&key);
        
        assert!(result.is_none());
        assert_eq!(cache.statistics().misses, 1);
    }

    #[test]
    fn test_cache_eviction() {
        let config = CacheConfig {
            max_entries: 2,
            max_size_bytes: 1024 * 1024,
            ttl: Duration::from_secs(300),
            enable_lru: true,
        };
        let mut cache = QueryCache::with_config(config);

        // Add entries
        for i in 0..3 {
            let key = CacheKey::new(&format!("SELECT {}", i), b"");
            let result = QueryResult {
                columns: vec!["id".to_string()],
                rows: vec![vec![Value::new(i.to_string().as_bytes())]],
                affected_rows: 1,
            };
            cache.put(key, result).unwrap();
        }

        // Should have evicted one entry
        assert_eq!(cache.entry_count(), 2);
        assert!(cache.statistics().evictions > 0);
    }

    #[test]
    fn test_hit_rate() {
        let mut cache = QueryCache::new();
        
        let key = CacheKey::new("SELECT * FROM users", b"");
        let result = QueryResult {
            columns: vec!["id".to_string()],
            rows: vec![],
            affected_rows: 0,
        };

        cache.put(key.clone(), result).unwrap();
        
        // 2 hits
        cache.get(&key);
        cache.get(&key);
        
        // 1 miss
        let other_key = CacheKey::new("SELECT * FROM orders", b"");
        cache.get(&other_key);

        // Hit rate should be 2/3
        assert!((cache.hit_rate() - 0.666).abs() < 0.01);
    }
}

