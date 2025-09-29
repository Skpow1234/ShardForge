//! Iterator interface for range queries

use async_trait::async_trait;
use shardforge_core::{Key, Result, Value};

/// Iterator for key-value pairs
#[async_trait]
pub trait StorageIterator: Send + Sync {
    /// Get the current key
    fn current_key(&self) -> Option<&Key>;

    /// Get the current value
    fn current_value(&self) -> Option<&Value>;

    /// Move to the next key-value pair
    async fn next(&mut self) -> Result<bool>;

    /// Check if the iterator is valid
    fn is_valid(&self) -> bool;

    /// Seek to a specific key or the first key >= the given key
    async fn seek(&mut self, key: &Key) -> Result<()>;

    /// Seek to the first key
    async fn seek_to_first(&mut self) -> Result<()>;

    /// Seek to the last key
    async fn seek_to_last(&mut self) -> Result<()>;
}

/// Direction for iteration
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IterationDirection {
    Forward,
    Backward,
}

/// Iterator options for range queries
#[derive(Debug, Clone)]
pub struct IteratorOptions {
    /// Start key (inclusive)
    pub start_key: Option<Key>,
    /// End key (exclusive)
    pub end_key: Option<Key>,
    /// Direction of iteration
    pub direction: IterationDirection,
    /// Maximum number of keys to return
    pub limit: Option<usize>,
    /// Include start key
    pub inclusive_start: bool,
    /// Include end key
    pub inclusive_end: bool,
}

impl Default for IteratorOptions {
    fn default() -> Self {
        Self {
            start_key: None,
            end_key: None,
            direction: IterationDirection::Forward,
            limit: None,
            inclusive_start: true,
            inclusive_end: false,
        }
    }
}

impl IteratorOptions {
    /// Create iterator for a specific key range
    pub fn range(start: Key, end: Key) -> Self {
        Self {
            start_key: Some(start),
            end_key: Some(end),
            ..Default::default()
        }
    }

    /// Create iterator starting from a specific key
    pub fn from(key: Key) -> Self {
        Self {
            start_key: Some(key),
            ..Default::default()
        }
    }

    /// Create iterator for all keys
    pub fn all() -> Self {
        Self::default()
    }

    /// Set limit on number of keys
    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = Some(limit);
        self
    }

    /// Set iteration direction
    pub fn with_direction(mut self, direction: IterationDirection) -> Self {
        self.direction = direction;
        self
    }

    /// Check if a key should be included based on the options
    pub fn should_include(&self, key: &Key) -> bool {
        // Check start key
        if let Some(start_key) = &self.start_key {
            if self.inclusive_start {
                if key < start_key {
                    return false;
                }
            } else if key <= start_key {
                return false;
            }
        }

        // Check end key
        if let Some(end_key) = &self.end_key {
            if self.inclusive_end {
                if key > end_key {
                    return false;
                }
            } else if key >= end_key {
                return false;
            }
        }

        true
    }
}

/// Helper function to collect iterator results into a vector
pub async fn collect_iterator(
    mut iter: Box<dyn StorageIterator>,
    options: IteratorOptions,
) -> Result<Vec<(Key, Value)>> {
    let mut results = Vec::new();

    // Seek to start position
    if let Some(start_key) = &options.start_key {
        iter.seek(start_key).await?;
    } else {
        iter.seek_to_first().await?;
    }

    // Iterate through results
    while iter.is_valid() {
        if let (Some(key), Some(value)) = (iter.current_key(), iter.current_value()) {
            if options.should_include(key) {
                results.push((key.clone(), value.clone()));
            } else if options.direction == IterationDirection::Forward
                && key >= options.end_key.as_ref().unwrap()
            {
                break;
            }
        }

        // Check limit
        if let Some(limit) = options.limit {
            if results.len() >= limit {
                break;
            }
        }

        if !iter.next().await? {
            break;
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;
    use shardforge_core::Key;

    #[test]
    fn test_iterator_options_should_include() {
        let options = IteratorOptions::range(Key::from_string("key1"), Key::from_string("key5"));

        assert!(!options.should_include(&Key::from_string("key0")));
        assert!(options.should_include(&Key::from_string("key1")));
        assert!(options.should_include(&Key::from_string("key2")));
        assert!(options.should_include(&Key::from_string("key4")));
        assert!(!options.should_include(&Key::from_string("key5")));
        assert!(!options.should_include(&Key::from_string("key6")));
    }

    #[test]
    fn test_iterator_options_inclusive_end() {
        let mut options =
            IteratorOptions::range(Key::from_string("key1"), Key::from_string("key5"));
        options.inclusive_end = true;

        assert!(options.should_include(&Key::from_string("key5")));
        assert!(!options.should_include(&Key::from_string("key6")));
    }
}
