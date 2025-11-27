//! Query planning and optimization

use shardforge_core::{Result, ShardForgeError};

use crate::sql::ast::*;
use crate::sql::executor::SchemaCatalog;
use crate::sql::statistics::{CostModel, StatisticsCollector};

/// Query planner
pub struct QueryPlanner {
    catalog: SchemaCatalog,
}

/// Logical query plan
#[derive(Debug, Clone)]
pub enum LogicalPlan {
    TableScan {
        table_name: String,
        projection: Option<Vec<String>>,
        filter: Option<Expression>,
    },
    Insert {
        table_name: String,
        columns: Option<Vec<String>>,
        values: Vec<Vec<Expression>>,
    },
    CreateTable {
        name: String,
        columns: Vec<ColumnDef>,
        constraints: Vec<TableConstraint>,
        if_not_exists: bool,
    },
    DropTable {
        name: String,
        if_exists: bool,
        cascade: bool,
    },
}

/// Physical query plan
#[derive(Debug, Clone)]
pub enum PhysicalPlan {
    SeqScan {
        table_name: String,
        projection: Option<Vec<String>>,
        filter: Option<Expression>,
    },
    IndexScan {
        table_name: String,
        index_name: String,
        key_conditions: Vec<Expression>,
        projection: Option<Vec<String>>,
    },
    Insert {
        table_name: String,
        columns: Option<Vec<String>>,
        values: Vec<Vec<Expression>>,
    },
    CreateTable {
        name: String,
        columns: Vec<ColumnDef>,
        constraints: Vec<TableConstraint>,
        if_not_exists: bool,
    },
    DropTable {
        name: String,
        if_exists: bool,
        cascade: bool,
    },
}

/// Query optimizer
pub struct QueryOptimizer {
    /// Statistics collector
    statistics: StatisticsCollector,
    /// Cost model
    cost_model: CostModel,
}

/// Cost estimation
#[derive(Debug, Clone)]
pub struct Cost {
    pub cpu_cost: f64,
    pub io_cost: f64,
    pub network_cost: f64,
    pub total_cost: f64,
}

impl QueryPlanner {
    /// Create a new query planner
    pub fn new(catalog: SchemaCatalog) -> Self {
        Self { catalog }
    }

    /// Create a logical plan from a SQL statement
    pub fn plan(&self, statement: Statement) -> Result<LogicalPlan> {
        match statement {
            Statement::Select(stmt) => self.plan_select(stmt),
            Statement::Insert(stmt) => self.plan_insert(stmt),
            Statement::CreateTable(stmt) => self.plan_create_table(stmt),
            Statement::DropTable(stmt) => self.plan_drop_table(stmt),
            _ => Err(ShardForgeError::Query {
                message: "Statement type not supported in planning".to_string(),
            }),
        }
    }

    /// Plan a SELECT statement
    fn plan_select(&self, stmt: SelectStatement) -> Result<LogicalPlan> {
        let table_name = stmt.from.ok_or_else(|| ShardForgeError::Query {
            message: "FROM clause is required".to_string(),
        })?;

        // Validate table exists
        if self.catalog.get_table(&table_name).is_none() {
            return Err(ShardForgeError::Query {
                message: format!("Table '{}' does not exist", table_name),
            });
        }

        // Extract projection columns
        let projection =
            if stmt.columns.len() == 1 && matches!(stmt.columns[0], SelectItem::Wildcard) {
                None // Wildcard means all columns
            } else {
                let mut columns = Vec::new();
                for item in stmt.columns {
                    match item {
                        SelectItem::Expression { expr, alias } => {
                            if let Expression::Column(column_name) = expr {
                                columns.push(alias.unwrap_or(column_name));
                            } else {
                                return Err(ShardForgeError::Query {
                                    message: "Complex expressions not yet supported in SELECT"
                                        .to_string(),
                                });
                            }
                        }
                        SelectItem::Wildcard => {
                            return Err(ShardForgeError::Query {
                                message: "Cannot mix wildcard with specific columns".to_string(),
                            });
                        }
                    }
                }
                Some(columns)
            };

        Ok(LogicalPlan::TableScan {
            table_name,
            projection,
            filter: stmt.where_clause,
        })
    }

    /// Plan an INSERT statement
    fn plan_insert(&self, stmt: InsertStatement) -> Result<LogicalPlan> {
        // Validate table exists
        if self.catalog.get_table(&stmt.table_name).is_none() {
            return Err(ShardForgeError::Query {
                message: format!("Table '{}' does not exist", stmt.table_name),
            });
        }

        Ok(LogicalPlan::Insert {
            table_name: stmt.table_name,
            columns: stmt.columns,
            values: stmt.values,
        })
    }

    /// Plan a CREATE TABLE statement
    fn plan_create_table(&self, stmt: CreateTableStatement) -> Result<LogicalPlan> {
        Ok(LogicalPlan::CreateTable {
            name: stmt.name,
            columns: stmt.columns,
            constraints: stmt.constraints,
            if_not_exists: stmt.if_not_exists,
        })
    }

    /// Plan a DROP TABLE statement
    fn plan_drop_table(&self, stmt: DropTableStatement) -> Result<LogicalPlan> {
        Ok(LogicalPlan::DropTable {
            name: stmt.name,
            if_exists: stmt.if_exists,
            cascade: stmt.cascade,
        })
    }
}

impl QueryOptimizer {
    /// Create a new query optimizer
    pub fn new() -> Self {
        Self {
            statistics: StatisticsCollector::new(),
            cost_model: CostModel::new(),
        }
    }

    /// Create a query optimizer with custom statistics
    pub fn with_statistics(statistics: StatisticsCollector) -> Self {
        Self {
            statistics,
            cost_model: CostModel::new(),
        }
    }

    /// Get a reference to the statistics collector
    pub fn statistics(&self) -> &StatisticsCollector {
        &self.statistics
    }

    /// Get a mutable reference to the statistics collector
    pub fn statistics_mut(&mut self) -> &mut StatisticsCollector {
        &mut self.statistics
    }

    /// Optimize a logical plan to create a physical plan
    pub fn optimize(&self, logical_plan: LogicalPlan) -> Result<PhysicalPlan> {
        match logical_plan {
            LogicalPlan::TableScan { table_name, projection, filter } => {
                self.optimize_table_scan(table_name, projection, filter)
            }
            LogicalPlan::Insert { table_name, columns, values } => {
                Ok(PhysicalPlan::Insert { table_name, columns, values })
            }
            LogicalPlan::CreateTable {
                name,
                columns,
                constraints,
                if_not_exists,
            } => Ok(PhysicalPlan::CreateTable {
                name,
                columns,
                constraints,
                if_not_exists,
            }),
            LogicalPlan::DropTable { name, if_exists, cascade } => {
                Ok(PhysicalPlan::DropTable { name, if_exists, cascade })
            }
        }
    }

    /// Optimize a table scan operation
    fn optimize_table_scan(
        &self,
        table_name: String,
        projection: Option<Vec<String>>,
        filter: Option<Expression>,
    ) -> Result<PhysicalPlan> {
        // Check if we have statistics for this table
        if let Some(table_stats) = self.statistics.get_table_stats(&table_name) {
            // Estimate selectivity of the filter
            let selectivity = if let Some(ref filter_expr) = filter {
                self.estimate_filter_selectivity(&table_name, filter_expr)
            } else {
                1.0
            };

            // Estimate costs for different access methods
            let seq_scan_cost = self.cost_model.estimate_seq_scan_cost(
                table_stats.row_count,
                table_stats.avg_row_size,
            );

            let index_scan_cost = self.cost_model.estimate_index_scan_cost(
                selectivity,
                table_stats.row_count,
            );

            // Choose the cheaper option (simplified - assumes an index exists)
            if selectivity < 0.2 && index_scan_cost < seq_scan_cost {
                // TODO: Implement actual index selection
                Ok(PhysicalPlan::SeqScan {
                    table_name,
                    projection,
                    filter,
                })
            } else {
                Ok(PhysicalPlan::SeqScan {
                    table_name,
                    projection,
                    filter,
                })
            }
        } else {
            // No statistics available, default to sequential scan
            Ok(PhysicalPlan::SeqScan {
                table_name,
                projection,
                filter,
            })
        }
    }

    /// Estimate the selectivity of a filter expression
    fn estimate_filter_selectivity(&self, table_name: &str, filter: &Expression) -> f64 {
        match filter {
            Expression::BinaryOp { left, op, right } => {
                if let (Expression::Column(col_name), Expression::Literal(_)) = (left.as_ref(), right.as_ref()) {
                    let operator = match op {
                        BinaryOperator::Equal => "=",
                        BinaryOperator::NotEqual => "!=",
                        BinaryOperator::LessThan => "<",
                        BinaryOperator::LessThanOrEqual => "<=",
                        BinaryOperator::GreaterThan => ">",
                        BinaryOperator::GreaterThanOrEqual => ">=",
                        _ => "=",
                    };
                    
                    self.statistics.estimate_selectivity(table_name, col_name, operator, b"")
                } else if matches!(op, BinaryOperator::And) {
                    // For AND, multiply selectivities
                    let left_sel = self.estimate_filter_selectivity(table_name, left);
                    let right_sel = self.estimate_filter_selectivity(table_name, right);
                    left_sel * right_sel
                } else if matches!(op, BinaryOperator::Or) {
                    // For OR, use inclusion-exclusion principle
                    let left_sel = self.estimate_filter_selectivity(table_name, left);
                    let right_sel = self.estimate_filter_selectivity(table_name, right);
                    left_sel + right_sel - (left_sel * right_sel)
                } else {
                    0.5 // Default selectivity
                }
            }
            _ => 0.5, // Default selectivity
        }
    }

    /// Estimate the cost of a physical plan
    pub fn estimate_cost(&self, plan: &PhysicalPlan) -> Cost {
        match plan {
            PhysicalPlan::SeqScan { table_name, filter, .. } => {
                if let Some(table_stats) = self.statistics.get_table_stats(table_name) {
                    let cpu_cost = self.cost_model.estimate_seq_scan_cost(
                        table_stats.row_count,
                        table_stats.avg_row_size,
                    );
                    
                    let selectivity = if let Some(filter_expr) = filter {
                        self.estimate_filter_selectivity(table_name, filter_expr)
                    } else {
                        1.0
                    };

                    Cost {
                        cpu_cost,
                        io_cost: cpu_cost * 0.5,
                        network_cost: 0.0,
                        total_cost: cpu_cost * 1.5 * selectivity,
                    }
                } else {
                    Cost {
                        cpu_cost: 100.0,
                        io_cost: 50.0,
                        network_cost: 0.0,
                        total_cost: 150.0,
                    }
                }
            }
            PhysicalPlan::IndexScan { table_name, .. } => {
                if let Some(table_stats) = self.statistics.get_table_stats(table_name) {
                    let cpu_cost = self.cost_model.estimate_index_scan_cost(0.1, table_stats.row_count);
                    Cost {
                        cpu_cost,
                        io_cost: cpu_cost * 0.3,
                        network_cost: 0.0,
                        total_cost: cpu_cost * 1.3,
                    }
                } else {
                    Cost {
                        cpu_cost: 50.0,
                        io_cost: 20.0,
                        network_cost: 0.0,
                        total_cost: 70.0,
                    }
                }
            }
            _ => Cost {
                cpu_cost: 10.0,
                io_cost: 5.0,
                network_cost: 0.0,
                total_cost: 15.0,
            },
        }
    }

    /// Check if an index can be used for a given filter condition
    pub fn can_use_index(&self, filter: &Expression, index_columns: &[String]) -> bool {
        // Check if the filter references the index columns
        match filter {
            Expression::BinaryOp { left, op, .. } => {
                if let Expression::Column(col_name) = left.as_ref() {
                    // Check if column is the first column of the index
                    if let Some(first_col) = index_columns.first() {
                        if first_col == col_name {
                            // Check if operator is index-compatible
                            return matches!(
                                op,
                                BinaryOperator::Equal
                                    | BinaryOperator::LessThan
                                    | BinaryOperator::LessThanOrEqual
                                    | BinaryOperator::GreaterThan
                                    | BinaryOperator::GreaterThanOrEqual
                            );
                        }
                    }
                }
                false
            }
            _ => false,
        }
    }
}

impl Default for QueryOptimizer {
    fn default() -> Self {
        Self::new()
    }
}

impl Cost {
    /// Create a new cost with all components set to zero
    pub fn zero() -> Self {
        Self {
            cpu_cost: 0.0,
            io_cost: 0.0,
            network_cost: 0.0,
            total_cost: 0.0,
        }
    }

    /// Add another cost to this one
    pub fn add(&mut self, other: &Cost) {
        self.cpu_cost += other.cpu_cost;
        self.io_cost += other.io_cost;
        self.network_cost += other.network_cost;
        self.total_cost += other.total_cost;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sql::executor::SchemaCatalog;

    #[test]
    fn test_plan_select() {
        let mut catalog = SchemaCatalog::new();

        // Add a test table to the catalog
        use crate::sql::executor::{ColumnSchema, TableSchema};
        let table_schema = TableSchema {
            name: "users".to_string(),
            columns: vec![ColumnSchema {
                name: "id".to_string(),
                data_type: DataType::Integer,
                nullable: false,
                default_value: None,
                auto_increment: true,
            }],
            primary_key: None,
            indexes: vec![],
        };
        catalog.add_table(table_schema);

        let planner = QueryPlanner::new(catalog);

        let select_stmt = SelectStatement {
            distinct: false,
            columns: vec![SelectItem::Wildcard],
            from: Some("users".to_string()),
            where_clause: None,
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
        };

        let plan = planner.plan(Statement::Select(select_stmt)).unwrap();

        match plan {
            LogicalPlan::TableScan { table_name, projection, filter } => {
                assert_eq!(table_name, "users");
                assert!(projection.is_none()); // Wildcard should result in None
                assert!(filter.is_none());
            }
            _ => panic!("Expected TableScan plan"),
        }
    }

    #[test]
    fn test_optimize_table_scan() {
        let optimizer = QueryOptimizer::new();

        let logical_plan = LogicalPlan::TableScan {
            table_name: "users".to_string(),
            projection: None,
            filter: None,
        };

        let physical_plan = optimizer.optimize(logical_plan).unwrap();

        match physical_plan {
            PhysicalPlan::SeqScan { table_name, projection, filter } => {
                assert_eq!(table_name, "users");
                assert!(projection.is_none());
                assert!(filter.is_none());
            }
            _ => panic!("Expected SeqScan plan"),
        }
    }

    #[test]
    fn test_cost_estimation() {
        let optimizer = QueryOptimizer::new();

        let plan = PhysicalPlan::SeqScan {
            table_name: "users".to_string(),
            projection: None,
            filter: None,
        };

        let cost = optimizer.estimate_cost(&plan);
        assert!(cost.total_cost > 0.0);
    }
}
