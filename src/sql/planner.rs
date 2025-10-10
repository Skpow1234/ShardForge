//! Query planning and optimization

use shardforge_core::{Result, ShardForgeError};

use crate::sql::ast::*;
use crate::sql::executor::{QueryResult, SchemaCatalog};

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
pub struct QueryOptimizer;

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
        let projection = if stmt.columns.len() == 1 && matches!(stmt.columns[0], SelectItem::Wildcard) {
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
                                message: "Complex expressions not yet supported in SELECT".to_string(),
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
        Self
    }

    /// Optimize a logical plan to create a physical plan
    pub fn optimize(&self, logical_plan: LogicalPlan) -> Result<PhysicalPlan> {
        match logical_plan {
            LogicalPlan::TableScan { table_name, projection, filter } => {
                // For now, always use sequential scan
                // In the future, we'll check for indexes and use them when beneficial
                Ok(PhysicalPlan::SeqScan {
                    table_name,
                    projection,
                    filter,
                })
            }
            LogicalPlan::Insert { table_name, columns, values } => {
                Ok(PhysicalPlan::Insert {
                    table_name,
                    columns,
                    values,
                })
            }
            LogicalPlan::CreateTable { name, columns, constraints, if_not_exists } => {
                Ok(PhysicalPlan::CreateTable {
                    name,
                    columns,
                    constraints,
                    if_not_exists,
                })
            }
            LogicalPlan::DropTable { name, if_exists, cascade } => {
                Ok(PhysicalPlan::DropTable {
                    name,
                    if_exists,
                    cascade,
                })
            }
        }
    }

    /// Estimate the cost of a physical plan
    pub fn estimate_cost(&self, _plan: &PhysicalPlan) -> Cost {
        // Simplified cost estimation
        Cost {
            cpu_cost: 100.0,
            io_cost: 50.0,
            network_cost: 0.0,
            total_cost: 150.0,
        }
    }

    /// Check if an index can be used for a given filter condition
    pub fn can_use_index(&self, _filter: &Expression, _index_columns: &[String]) -> bool {
        // Simplified index usage check
        // In a real implementation, this would analyze the filter expression
        // to see if it matches the index columns
        false
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
        use crate::sql::executor::{TableSchema, ColumnSchema};
        let table_schema = TableSchema {
            name: "users".to_string(),
            columns: vec![
                ColumnSchema {
                    name: "id".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                    default_value: None,
                    auto_increment: true,
                },
            ],
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
