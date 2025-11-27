//! Aggregate function implementation

use shardforge_core::{Result, ShardForgeError, Value};
use std::collections::HashMap;

use crate::sql::ast::Expression;

/// Aggregate function types
#[derive(Debug, Clone, PartialEq)]
pub enum AggregateFunction {
    Count,
    Sum,
    Avg,
    Min,
    Max,
}

impl AggregateFunction {
    /// Parse aggregate function name
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_uppercase().as_str() {
            "COUNT" => Some(AggregateFunction::Count),
            "SUM" => Some(AggregateFunction::Sum),
            "AVG" => Some(AggregateFunction::Avg),
            "MIN" => Some(AggregateFunction::Min),
            "MAX" => Some(AggregateFunction::Max),
            _ => None,
        }
    }
}

/// Aggregate accumulator for computing aggregate values
#[derive(Debug, Clone)]
pub struct AggregateAccumulator {
    function: AggregateFunction,
    count: u64,
    sum: f64,
    min: Option<Vec<u8>>,
    max: Option<Vec<u8>>,
}

impl AggregateAccumulator {
    /// Create a new accumulator for the given function
    pub fn new(function: AggregateFunction) -> Self {
        Self {
            function,
            count: 0,
            sum: 0.0,
            min: None,
            max: None,
        }
    }

    /// Accumulate a value
    pub fn accumulate(&mut self, value: &Value) -> Result<()> {
        self.count += 1;

        match self.function {
            AggregateFunction::Count => {
                // Count just increments, already done above
            }
            AggregateFunction::Sum | AggregateFunction::Avg => {
                // Try to parse value as number
                let num = self.value_to_number(value)?;
                self.sum += num;
            }
            AggregateFunction::Min => {
                let val_bytes = value.as_ref().to_vec();
                if let Some(ref current_min) = self.min {
                    if val_bytes < *current_min {
                        self.min = Some(val_bytes);
                    }
                } else {
                    self.min = Some(val_bytes);
                }
            }
            AggregateFunction::Max => {
                let val_bytes = value.as_ref().to_vec();
                if let Some(ref current_max) = self.max {
                    if val_bytes > *current_max {
                        self.max = Some(val_bytes);
                    }
                } else {
                    self.max = Some(val_bytes);
                }
            }
        }

        Ok(())
    }

    /// Get the final aggregated value
    pub fn finalize(&self) -> Result<Value> {
        match self.function {
            AggregateFunction::Count => {
                Ok(Value::new(&self.count.to_le_bytes()))
            }
            AggregateFunction::Sum => {
                Ok(Value::new(&self.sum.to_le_bytes()))
            }
            AggregateFunction::Avg => {
                if self.count == 0 {
                    Ok(Value::new(&0.0_f64.to_le_bytes()))
                } else {
                    let avg = self.sum / self.count as f64;
                    Ok(Value::new(&avg.to_le_bytes()))
                }
            }
            AggregateFunction::Min => {
                if let Some(ref min_val) = self.min {
                    Ok(Value::new(min_val))
                } else {
                    Ok(Value::new(b""))
                }
            }
            AggregateFunction::Max => {
                if let Some(ref max_val) = self.max {
                    Ok(Value::new(max_val))
                } else {
                    Ok(Value::new(b""))
                }
            }
        }
    }

    /// Convert value to number for arithmetic operations
    fn value_to_number(&self, value: &Value) -> Result<f64> {
        let bytes = value.as_ref();

        // Try to parse as i64
        if bytes.len() == 8 {
            let int_val = i64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]));
            return Ok(int_val as f64);
        }

        // Try to parse as f64
        if bytes.len() == 8 {
            let float_val = f64::from_le_bytes(bytes.try_into().unwrap_or([0; 8]));
            return Ok(float_val);
        }

        // Try to parse as string
        if let Ok(s) = std::str::from_utf8(bytes) {
            if let Ok(num) = s.parse::<f64>() {
                return Ok(num);
            }
        }

        Err(ShardForgeError::Query {
            message: format!("Cannot convert value to number for aggregation"),
        })
    }
}

/// Group-by aggregation state
#[derive(Debug)]
pub struct GroupAggregationState {
    /// Map from group key to aggregate accumulators
    groups: HashMap<Vec<u8>, Vec<AggregateAccumulator>>,
    /// Group by column indices
    group_by_columns: Vec<usize>,
    /// Aggregate functions to compute
    aggregate_functions: Vec<(usize, AggregateFunction)>, // (column_index, function)
}

impl GroupAggregationState {
    /// Create a new group aggregation state
    pub fn new(
        group_by_columns: Vec<usize>,
        aggregate_functions: Vec<(usize, AggregateFunction)>,
    ) -> Self {
        Self {
            groups: HashMap::new(),
            group_by_columns,
            aggregate_functions,
        }
    }

    /// Accumulate a row
    pub fn accumulate_row(&mut self, row: &[Value]) -> Result<()> {
        // Compute group key
        let group_key = self.compute_group_key(row)?;

        // Get or create accumulators for this group
        let accumulators = self.groups.entry(group_key).or_insert_with(|| {
            self.aggregate_functions
                .iter()
                .map(|(_, func)| AggregateAccumulator::new(func.clone()))
                .collect()
        });

        // Accumulate values for each aggregate function
        for (i, (col_idx, _)) in self.aggregate_functions.iter().enumerate() {
            if *col_idx < row.len() {
                accumulators[i].accumulate(&row[*col_idx])?;
            }
        }

        Ok(())
    }

    /// Finalize and get all groups with their aggregated values
    pub fn finalize(self) -> Result<Vec<(Vec<Value>, Vec<Value>)>> {
        let mut results = Vec::new();

        for (group_key, accumulators) in self.groups {
            // Deserialize group key back to values
            let group_values = self.deserialize_group_key(&group_key)?;

            // Finalize all aggregates for this group
            let aggregate_values: Result<Vec<Value>> =
                accumulators.iter().map(|acc| acc.finalize()).collect();

            results.push((group_values, aggregate_values?));
        }

        Ok(results)
    }

    /// Compute a group key from row values
    fn compute_group_key(&self, row: &[Value]) -> Result<Vec<u8>> {
        let mut key = Vec::new();

        for &col_idx in &self.group_by_columns {
            if col_idx < row.len() {
                let val = row[col_idx].as_ref();
                // Store length + data
                key.extend_from_slice(&(val.len() as u32).to_le_bytes());
                key.extend_from_slice(val);
            }
        }

        Ok(key)
    }

    /// Deserialize group key back to values
    fn deserialize_group_key(&self, key: &[u8]) -> Result<Vec<Value>> {
        let mut values = Vec::new();
        let mut offset = 0;

        for _ in 0..self.group_by_columns.len() {
            if offset + 4 > key.len() {
                break;
            }

            let len = u32::from_le_bytes([
                key[offset],
                key[offset + 1],
                key[offset + 2],
                key[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + len > key.len() {
                break;
            }

            let val_data = &key[offset..offset + len];
            values.push(Value::new(val_data));
            offset += len;
        }

        Ok(values)
    }
}

/// Check if an expression contains aggregate functions
pub fn contains_aggregate(expr: &Expression) -> bool {
    match expr {
        Expression::Function { name, args } => {
            if AggregateFunction::from_name(name).is_some() {
                return true;
            }
            // Check arguments recursively
            args.iter().any(contains_aggregate)
        }
        Expression::BinaryOp { left, right, .. } => {
            contains_aggregate(left) || contains_aggregate(right)
        }
        Expression::UnaryOp { expr, .. } => contains_aggregate(expr),
        _ => false,
    }
}

/// Extract aggregate functions from a list of expressions
pub fn extract_aggregates(exprs: &[Expression]) -> Vec<(usize, AggregateFunction, Expression)> {
    let mut aggregates = Vec::new();

    for (idx, expr) in exprs.iter().enumerate() {
        if let Expression::Function { name, args } = expr {
            if let Some(func) = AggregateFunction::from_name(name) {
                // For simplicity, assume single argument or COUNT(*)
                let arg_expr = if args.is_empty() {
                    Expression::Literal(crate::sql::ast::Literal::Integer(1))
                } else {
                    args[0].clone()
                };
                aggregates.push((idx, func, arg_expr));
            }
        }
    }

    aggregates
}

