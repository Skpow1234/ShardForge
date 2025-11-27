//! Query statistics and cost estimation

use std::collections::HashMap;
use serde::{Deserialize, Serialize};

/// Table statistics for cost-based optimization
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableStatistics {
    /// Table name
    pub table_name: String,
    /// Estimated number of rows
    pub row_count: u64,
    /// Average row size in bytes
    pub avg_row_size: u64,
    /// Total table size in bytes
    pub total_size: u64,
    /// Column statistics
    pub column_stats: HashMap<String, ColumnStatistics>,
    /// Last update timestamp
    pub last_updated: std::time::SystemTime,
}

/// Column statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnStatistics {
    /// Column name
    pub column_name: String,
    /// Number of distinct values
    pub distinct_count: u64,
    /// Number of NULL values
    pub null_count: u64,
    /// Minimum value (serialized)
    pub min_value: Option<Vec<u8>>,
    /// Maximum value (serialized)
    pub max_value: Option<Vec<u8>>,
    /// Histogram for value distribution
    pub histogram: Option<Histogram>,
}

/// Value distribution histogram
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Histogram {
    /// Histogram buckets
    pub buckets: Vec<HistogramBucket>,
}

/// Histogram bucket
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistogramBucket {
    /// Lower bound (inclusive)
    pub lower_bound: Vec<u8>,
    /// Upper bound (exclusive)
    pub upper_bound: Vec<u8>,
    /// Number of rows in this bucket
    pub row_count: u64,
}

/// Statistics collector
pub struct StatisticsCollector {
    /// Statistics by table name
    statistics: HashMap<String, TableStatistics>,
}

impl StatisticsCollector {
    /// Create a new statistics collector
    pub fn new() -> Self {
        Self {
            statistics: HashMap::new(),
        }
    }

    /// Get statistics for a table
    pub fn get_table_stats(&self, table_name: &str) -> Option<&TableStatistics> {
        self.statistics.get(table_name)
    }

    /// Update statistics for a table
    pub fn update_table_stats(&mut self, stats: TableStatistics) {
        self.statistics.insert(stats.table_name.clone(), stats);
    }

    /// Estimate selectivity of a predicate
    pub fn estimate_selectivity(
        &self,
        table_name: &str,
        column_name: &str,
        operator: &str,
        _value: &[u8],
    ) -> f64 {
        if let Some(table_stats) = self.get_table_stats(table_name) {
            if let Some(col_stats) = table_stats.column_stats.get(column_name) {
                // Simplified selectivity estimation
                match operator {
                    "=" => {
                        if col_stats.distinct_count > 0 {
                            1.0 / col_stats.distinct_count as f64
                        } else {
                            0.1
                        }
                    }
                    "<" | "<=" | ">" | ">=" => 0.33, // Estimate 1/3 for range predicates
                    "!=" => {
                        if col_stats.distinct_count > 0 {
                            1.0 - (1.0 / col_stats.distinct_count as f64)
                        } else {
                            0.9
                        }
                    }
                    _ => 0.5, // Default selectivity
                }
            } else {
                0.5 // Default when no column stats available
            }
        } else {
            0.5 // Default when no table stats available
        }
    }

    /// Estimate the number of rows returned by a query
    pub fn estimate_row_count(
        &self,
        table_name: &str,
        selectivity: f64,
    ) -> u64 {
        if let Some(table_stats) = self.get_table_stats(table_name) {
            (table_stats.row_count as f64 * selectivity) as u64
        } else {
            1000 // Default estimate
        }
    }
}

impl Default for StatisticsCollector {
    fn default() -> Self {
        Self::new()
    }
}

impl TableStatistics {
    /// Create new table statistics with default values
    pub fn new(table_name: String) -> Self {
        Self {
            table_name,
            row_count: 0,
            avg_row_size: 0,
            total_size: 0,
            column_stats: HashMap::new(),
            last_updated: std::time::SystemTime::now(),
        }
    }

    /// Update row count
    pub fn set_row_count(&mut self, count: u64) {
        self.row_count = count;
        self.last_updated = std::time::SystemTime::now();
    }

    /// Add column statistics
    pub fn add_column_stats(&mut self, stats: ColumnStatistics) {
        self.column_stats.insert(stats.column_name.clone(), stats);
        self.last_updated = std::time::SystemTime::now();
    }
}

impl ColumnStatistics {
    /// Create new column statistics with default values
    pub fn new(column_name: String) -> Self {
        Self {
            column_name,
            distinct_count: 0,
            null_count: 0,
            min_value: None,
            max_value: None,
            histogram: None,
        }
    }

    /// Set distinct count
    pub fn set_distinct_count(&mut self, count: u64) {
        self.distinct_count = count;
    }

    /// Set null count
    pub fn set_null_count(&mut self, count: u64) {
        self.null_count = count;
    }

    /// Set min and max values
    pub fn set_min_max(&mut self, min: Vec<u8>, max: Vec<u8>) {
        self.min_value = Some(min);
        self.max_value = Some(max);
    }
}

/// Cost estimation model
#[derive(Debug, Clone)]
pub struct CostModel {
    /// Cost per sequential I/O operation
    pub seq_io_cost: f64,
    /// Cost per random I/O operation
    pub random_io_cost: f64,
    /// Cost per CPU operation
    pub cpu_cost: f64,
    /// Cost per network operation
    pub network_cost: f64,
}

impl CostModel {
    /// Create a new cost model with default values
    pub fn new() -> Self {
        Self {
            seq_io_cost: 1.0,
            random_io_cost: 4.0,
            cpu_cost: 0.01,
            network_cost: 10.0,
        }
    }

    /// Estimate cost of a sequential scan
    pub fn estimate_seq_scan_cost(&self, row_count: u64, row_size: u64) -> f64 {
        let pages = (row_count * row_size) / 8192; // Assuming 8KB page size
        (pages as f64 * self.seq_io_cost) + (row_count as f64 * self.cpu_cost)
    }

    /// Estimate cost of an index scan
    pub fn estimate_index_scan_cost(&self, selectivity: f64, row_count: u64) -> f64 {
        let rows_to_fetch = (row_count as f64 * selectivity) as u64;
        (rows_to_fetch as f64 * self.random_io_cost) + (row_count as f64 * self.cpu_cost)
    }

    /// Estimate cost of a hash join
    pub fn estimate_hash_join_cost(&self, left_rows: u64, right_rows: u64) -> f64 {
        // Cost to build hash table + cost to probe
        (left_rows as f64 * self.cpu_cost * 2.0) + (right_rows as f64 * self.cpu_cost)
    }

    /// Estimate cost of a nested loop join
    pub fn estimate_nested_loop_cost(&self, outer_rows: u64, inner_rows: u64) -> f64 {
        outer_rows as f64 * inner_rows as f64 * self.cpu_cost
    }

    /// Estimate cost of sorting
    pub fn estimate_sort_cost(&self, row_count: u64) -> f64 {
        if row_count == 0 {
            return 0.0;
        }
        // O(n log n) complexity
        row_count as f64 * (row_count as f64).log2() * self.cpu_cost
    }

    /// Estimate cost of aggregation
    pub fn estimate_aggregation_cost(&self, row_count: u64, group_count: u64) -> f64 {
        // Hash table operations
        (row_count as f64 * self.cpu_cost * 2.0) + (group_count as f64 * self.cpu_cost)
    }
}

impl Default for CostModel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_statistics() {
        let mut stats = TableStatistics::new("users".to_string());
        stats.set_row_count(1000);
        
        let mut col_stats = ColumnStatistics::new("id".to_string());
        col_stats.set_distinct_count(1000);
        stats.add_column_stats(col_stats);

        assert_eq!(stats.row_count, 1000);
        assert!(stats.column_stats.contains_key("id"));
    }

    #[test]
    fn test_selectivity_estimation() {
        let mut collector = StatisticsCollector::new();
        
        let mut stats = TableStatistics::new("users".to_string());
        stats.set_row_count(1000);
        
        let mut col_stats = ColumnStatistics::new("status".to_string());
        col_stats.set_distinct_count(10);
        stats.add_column_stats(col_stats);
        
        collector.update_table_stats(stats);

        let selectivity = collector.estimate_selectivity("users", "status", "=", b"active");
        assert_eq!(selectivity, 0.1); // 1/10 distinct values
    }

    #[test]
    fn test_cost_estimation() {
        let cost_model = CostModel::new();
        
        let seq_scan_cost = cost_model.estimate_seq_scan_cost(1000, 100);
        assert!(seq_scan_cost > 0.0);
        
        let index_scan_cost = cost_model.estimate_index_scan_cost(0.1, 1000);
        assert!(index_scan_cost > 0.0);
        
        // Index scan should be cheaper for selective queries
        assert!(index_scan_cost < seq_scan_cost);
    }
}

