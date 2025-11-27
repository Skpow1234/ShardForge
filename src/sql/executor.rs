//! Query execution engine

use std::collections::HashMap;

use shardforge_core::{Key, Result, ShardForgeError, Value};
use shardforge_storage::{MVCCStorage, StorageEngine};

use crate::sql::aggregation::{contains_aggregate, extract_aggregates, GroupAggregationState};
use crate::sql::ast::*;

/// Query execution context
pub struct ExecutionContext {
    /// Storage engine for data access
    pub storage: Box<dyn StorageEngine>,
    /// MVCC layer for transaction support
    pub mvcc: MVCCStorage,
    /// Schema catalog
    pub catalog: SchemaCatalog,
}

/// Schema catalog for metadata management
pub struct SchemaCatalog {
    /// Table schemas by name
    tables: HashMap<String, TableSchema>,
}

/// Table schema definition
#[derive(Debug, Clone)]
pub struct TableSchema {
    pub name: String,
    pub columns: Vec<ColumnSchema>,
    pub primary_key: Option<Vec<String>>,
    pub indexes: Vec<IndexSchema>,
}

/// Column schema definition
#[derive(Debug, Clone)]
pub struct ColumnSchema {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default_value: Option<Expression>,
    pub auto_increment: bool,
}

/// Index schema definition
#[derive(Debug, Clone)]
pub struct IndexSchema {
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
    pub unique: bool,
}

/// Query executor
pub struct QueryExecutor {
    context: ExecutionContext,
}

/// Query execution result
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Value>>,
    pub affected_rows: u64,
}

impl SchemaCatalog {
    /// Create a new schema catalog
    pub fn new() -> Self {
        Self { tables: HashMap::new() }
    }

    /// Add a table schema
    pub fn add_table(&mut self, schema: TableSchema) {
        self.tables.insert(schema.name.clone(), schema);
    }

    /// Get a table schema
    pub fn get_table(&self, name: &str) -> Option<&TableSchema> {
        self.tables.get(name)
    }

    /// Remove a table schema
    pub fn remove_table(&mut self, name: &str) -> Option<TableSchema> {
        self.tables.remove(name)
    }

    /// List all table names
    pub fn list_tables(&self) -> Vec<String> {
        self.tables.keys().cloned().collect()
    }
}

impl QueryExecutor {
    /// Create a new query executor
    pub fn new(context: ExecutionContext) -> Self {
        Self { context }
    }

    /// Execute a SQL statement
    pub async fn execute(&mut self, statement: Statement) -> Result<QueryResult> {
        match statement {
            Statement::CreateTable(stmt) => self.execute_create_table(stmt).await,
            Statement::DropTable(stmt) => self.execute_drop_table(stmt).await,
            Statement::CreateIndex(stmt) => self.execute_create_index(stmt).await,
            Statement::DropIndex(stmt) => self.execute_drop_index(stmt).await,
            Statement::AlterTable(stmt) => self.execute_alter_table(stmt).await,
            Statement::Insert(stmt) => self.execute_insert(stmt).await,
            Statement::Update(stmt) => self.execute_update(stmt).await,
            Statement::Delete(stmt) => self.execute_delete(stmt).await,
            Statement::Select(stmt) => self.execute_select(stmt).await,
            _ => Err(ShardForgeError::Query {
                message: "Statement type not yet implemented".to_string(),
            }),
        }
    }

    /// Execute CREATE TABLE statement
    async fn execute_create_table(&mut self, stmt: CreateTableStatement) -> Result<QueryResult> {
        // Check if table already exists
        if self.context.catalog.get_table(&stmt.name).is_some() {
            if stmt.if_not_exists {
                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    affected_rows: 0,
                });
            } else {
                return Err(ShardForgeError::Query {
                    message: format!("Table '{}' already exists", stmt.name),
                });
            }
        }

        // Convert AST column definitions to schema
        let columns = stmt
            .columns
            .into_iter()
            .map(|col| ColumnSchema {
                name: col.name,
                data_type: col.data_type,
                nullable: col.nullable,
                default_value: col.default,
                auto_increment: col.auto_increment,
            })
            .collect();

        // Create table schema
        let table_schema = TableSchema {
            name: stmt.name.clone(),
            columns,
            primary_key: None, // TODO: Extract from constraints
            indexes: vec![],
        };

        // Add to catalog
        self.context.catalog.add_table(table_schema);

        // Store table metadata in storage engine
        let metadata_key = Key::new(format!("__table__{}", stmt.name).as_bytes());
        let metadata_value = Value::new(b"created"); // Simplified metadata
        self.context.storage.put(metadata_key, metadata_value).await?;

        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: 1,
        })
    }

    /// Execute DROP TABLE statement
    async fn execute_drop_table(&mut self, stmt: DropTableStatement) -> Result<QueryResult> {
        // Check if table exists
        if self.context.catalog.get_table(&stmt.name).is_none() {
            if stmt.if_exists {
                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    affected_rows: 0,
                });
            } else {
                return Err(ShardForgeError::Query {
                    message: format!("Table '{}' does not exist", stmt.name),
                });
            }
        }

        // Remove from catalog
        self.context.catalog.remove_table(&stmt.name);

        // Remove table metadata from storage
        let metadata_key = Key::new(format!("__table__{}", stmt.name).as_bytes());
        self.context.storage.delete(&metadata_key).await?;

        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: 1,
        })
    }

    /// Execute INSERT statement
    async fn execute_insert(&mut self, stmt: InsertStatement) -> Result<QueryResult> {
        // Get table schema
        let table_schema = self.context.catalog.get_table(&stmt.table_name).ok_or_else(|| {
            ShardForgeError::Query {
                message: format!("Table '{}' does not exist", stmt.table_name),
            }
        })?;

        let mut affected_rows = 0;

        // Process each row
        for (row_index, values) in stmt.values.iter().enumerate() {
            // Validate column count
            let expected_columns = if let Some(ref columns) = stmt.columns {
                columns.len()
            } else {
                table_schema.columns.len()
            };

            if values.len() != expected_columns {
                return Err(ShardForgeError::Query {
                    message: format!(
                        "Expected {} values, got {} for row {}",
                        expected_columns,
                        values.len(),
                        row_index
                    ),
                });
            }

            // Generate primary key (simplified - use row index for now)
            let primary_key = Key::new(format!("{}:{}", stmt.table_name, row_index).as_bytes());

            // Convert expressions to values (simplified evaluation)
            let mut row_data = Vec::new();
            for expr in values {
                let value = self.evaluate_expression(expr)?;
                row_data.push(value);
            }

            // Serialize row data (simplified)
            let serialized = self.serialize_row(&row_data)?;
            let storage_value = Value::new(&serialized);

            // Store in storage engine
            self.context.storage.put(primary_key, storage_value).await?;
            affected_rows += 1;
        }

        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows,
        })
    }

    /// Execute UPDATE statement
    async fn execute_update(&mut self, stmt: UpdateStatement) -> Result<QueryResult> {
        // Get table schema
        let table_schema = self.context.catalog.get_table(&stmt.table_name).ok_or_else(|| {
            ShardForgeError::Query {
                message: format!("Table '{}' does not exist", stmt.table_name),
            }
        })?;

        let mut affected_rows = 0;

        // Scan all rows in the table
        let prefix = Key::new(format!("{}:", stmt.table_name).as_bytes());
        let all_keys = self.scan_table_keys(&stmt.table_name).await?;

        for key in all_keys {
            // Get row data
            if let Some(storage_value) = self.context.storage.get(&key).await? {
                let row_data = self.deserialize_row(storage_value.as_ref())?;

                // Check WHERE clause
                if let Some(ref where_clause) = stmt.where_clause {
                    if !self.evaluate_condition(where_clause, table_schema, &row_data)? {
                        continue;
                    }
                }

                // Apply updates
                let mut updated_row = row_data.clone();
                for assignment in &stmt.assignments {
                    let column_index = table_schema
                        .columns
                        .iter()
                        .position(|c| c.name == assignment.column)
                        .ok_or_else(|| ShardForgeError::Query {
                            message: format!("Column '{}' not found", assignment.column),
                        })?;

                    let new_value = self.evaluate_expression(&assignment.value)?;
                    updated_row[column_index] = new_value;
                }

                // Store updated row
                let serialized = self.serialize_row(&updated_row)?;
                let storage_value = Value::new(&serialized);
                self.context.storage.put(key, storage_value).await?;
                affected_rows += 1;
            }
        }

        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows,
        })
    }

    /// Execute DELETE statement
    async fn execute_delete(&mut self, stmt: DeleteStatement) -> Result<QueryResult> {
        // Get table schema
        let table_schema = self.context.catalog.get_table(&stmt.table_name).ok_or_else(|| {
            ShardForgeError::Query {
                message: format!("Table '{}' does not exist", stmt.table_name),
            }
        })?;

        let mut affected_rows = 0;

        // Scan all rows in the table
        let all_keys = self.scan_table_keys(&stmt.table_name).await?;

        for key in all_keys {
            // Get row data for WHERE clause evaluation
            if let Some(storage_value) = self.context.storage.get(&key).await? {
                let row_data = self.deserialize_row(storage_value.as_ref())?;

                // Check WHERE clause
                if let Some(ref where_clause) = stmt.where_clause {
                    if !self.evaluate_condition(where_clause, table_schema, &row_data)? {
                        continue;
                    }
                }

                // Delete the row
                self.context.storage.delete(&key).await?;
                affected_rows += 1;
            }
        }

        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows,
        })
    }

    /// Execute SELECT statement with full table scan
    async fn execute_select(&mut self, stmt: SelectStatement) -> Result<QueryResult> {
        // Get table name (simplified - single table queries only)
        let table_name = stmt.from.ok_or_else(|| ShardForgeError::Query {
            message: "FROM clause is required".to_string(),
        })?;

        // Get table schema
        let table_schema =
            self.context
                .catalog
                .get_table(&table_name)
                .ok_or_else(|| ShardForgeError::Query {
                    message: format!("Table '{}' does not exist", table_name),
                })?;

        // Check if this is an aggregate query
        let has_aggregates = stmt.columns.iter().any(|item| {
            if let SelectItem::Expression { expr, .. } = item {
                contains_aggregate(expr)
            } else {
                false
            }
        });

        if has_aggregates || !stmt.group_by.is_empty() {
            self.execute_select_with_aggregation(stmt, table_schema, &table_name)
                .await
        } else {
            self.execute_select_simple(stmt, table_schema, &table_name)
                .await
        }
    }

    /// Execute a simple SELECT without aggregation
    async fn execute_select_simple(
        &mut self,
        stmt: SelectStatement,
        table_schema: &TableSchema,
        table_name: &str,
    ) -> Result<QueryResult> {
        let selected_column_names = self.get_selected_column_names(&stmt.columns, table_schema)?;
        let mut result_rows = Vec::new();

        // Scan all rows in the table
        let all_keys = self.scan_table_keys(table_name).await?;

        for key in all_keys {
            if let Some(storage_value) = self.context.storage.get(&key).await? {
                let row_data = self.deserialize_row(storage_value.as_ref())?;

                // Apply WHERE filter
                if let Some(ref where_clause) = stmt.where_clause {
                    if !self.evaluate_condition(where_clause, table_schema, &row_data)? {
                        continue;
                    }
                }

                // Apply column selection
                let (_, selected_data) =
                    self.apply_column_selection(&stmt.columns, table_schema, &row_data)?;

                result_rows.push(selected_data);
            }
        }

        // Apply DISTINCT
        if stmt.distinct {
            result_rows.sort();
            result_rows.dedup();
        }

        // Apply ORDER BY
        if !stmt.order_by.is_empty() {
            result_rows.sort_by(|a, b| {
                for order_item in &stmt.order_by {
                    // Simplified: compare by column index
                    if let Expression::Column(ref col_name) = order_item.expression {
                        if let Some(col_index) =
                            selected_column_names.iter().position(|n| n == col_name)
                        {
                            if col_index < a.len() && col_index < b.len() {
                                let cmp = a[col_index].as_ref().cmp(b[col_index].as_ref());
                                if cmp != std::cmp::Ordering::Equal {
                                    return match order_item.direction {
                                        OrderDirection::Asc => cmp,
                                        OrderDirection::Desc => cmp.reverse(),
                                    };
                                }
                            }
                        }
                    }
                }
                std::cmp::Ordering::Equal
            });
        }

        // Apply LIMIT and OFFSET
        let offset = stmt.offset.unwrap_or(0) as usize;
        let limit = stmt.limit.map(|l| l as usize);

        let final_rows: Vec<_> = result_rows
            .into_iter()
            .skip(offset)
            .take(limit.unwrap_or(usize::MAX))
            .collect();

        let affected_rows = final_rows.len() as u64;

        Ok(QueryResult {
            columns: selected_column_names,
            rows: final_rows,
            affected_rows,
        })
    }

    /// Execute a SELECT with aggregation and GROUP BY
    async fn execute_select_with_aggregation(
        &mut self,
        stmt: SelectStatement,
        table_schema: &TableSchema,
        table_name: &str,
    ) -> Result<QueryResult> {
        // Extract expressions from SELECT items
        let mut select_exprs = Vec::new();
        let mut column_names = Vec::new();

        for item in &stmt.columns {
            match item {
                SelectItem::Expression { expr, alias } => {
                    select_exprs.push(expr.clone());
                    
                    // Generate column name
                    let col_name = if let Some(alias) = alias {
                        alias.clone()
                    } else {
                        match expr {
                            Expression::Function { name, .. } => name.clone(),
                            Expression::Column(col) => col.clone(),
                            _ => "expr".to_string(),
                        }
                    };
                    column_names.push(col_name);
                }
                SelectItem::Wildcard => {
                    return Err(ShardForgeError::Query {
                        message: "Cannot use * with aggregate functions".to_string(),
                    });
                }
            }
        }

        // Extract aggregate functions
        let aggregates = extract_aggregates(&select_exprs);

        // Determine GROUP BY columns
        let group_by_indices: Vec<usize> = stmt
            .group_by
            .iter()
            .filter_map(|expr| {
                if let Expression::Column(col_name) = expr {
                    table_schema.columns.iter().position(|c| c.name == *col_name)
                } else {
                    None
                }
            })
            .collect();

        // Extract aggregate column indices and functions
        let aggregate_funcs: Vec<(usize, crate::sql::aggregation::AggregateFunction)> =
            aggregates
                .iter()
                .filter_map(|(_, func, arg_expr)| {
                    if let Expression::Column(col_name) = arg_expr {
                        table_schema
                            .columns
                            .iter()
                            .position(|c| c.name == *col_name)
                            .map(|idx| (idx, func.clone()))
                    } else {
                        // For COUNT(*), use column 0 as a placeholder
                        Some((0, func.clone()))
                    }
                })
                .collect();

        // Create aggregation state
        let mut agg_state = GroupAggregationState::new(group_by_indices.clone(), aggregate_funcs);

        // Scan all rows and accumulate
        let all_keys = self.scan_table_keys(table_name).await?;

        for key in all_keys {
            if let Some(storage_value) = self.context.storage.get(&key).await? {
                let row_data = self.deserialize_row(storage_value.as_ref())?;

                // Apply WHERE filter
                if let Some(ref where_clause) = stmt.where_clause {
                    if !self.evaluate_condition(where_clause, table_schema, &row_data)? {
                        continue;
                    }
                }

                // Accumulate for aggregation
                agg_state.accumulate_row(&row_data)?;
            }
        }

        // Finalize aggregation
        let groups = agg_state.finalize()?;

        // Build result rows
        let mut result_rows = Vec::new();
        for (group_values, aggregate_values) in groups {
            let mut row = Vec::new();
            
            // Add GROUP BY columns
            for val in group_values {
                row.push(val);
            }
            
            // Add aggregate values
            for val in aggregate_values {
                row.push(val);
            }

            // Apply HAVING filter if present
            if let Some(ref having) = stmt.having {
                // Simplified HAVING evaluation - would need more context
                // For now, skip HAVING clause evaluation
            }

            result_rows.push(row);
        }

        // Apply ORDER BY
        if !stmt.order_by.is_empty() {
            result_rows.sort_by(|a, b| {
                for order_item in &stmt.order_by {
                    if let Expression::Column(ref col_name) = order_item.expression {
                        if let Some(col_index) = column_names.iter().position(|n| n == col_name) {
                            if col_index < a.len() && col_index < b.len() {
                                let cmp = a[col_index].as_ref().cmp(b[col_index].as_ref());
                                if cmp != std::cmp::Ordering::Equal {
                                    return match order_item.direction {
                                        OrderDirection::Asc => cmp,
                                        OrderDirection::Desc => cmp.reverse(),
                                    };
                                }
                            }
                        }
                    }
                }
                std::cmp::Ordering::Equal
            });
        }

        // Apply LIMIT and OFFSET
        let offset = stmt.offset.unwrap_or(0) as usize;
        let limit = stmt.limit.map(|l| l as usize);

        let final_rows: Vec<_> = result_rows
            .into_iter()
            .skip(offset)
            .take(limit.unwrap_or(usize::MAX))
            .collect();

        let affected_rows = final_rows.len() as u64;

        Ok(QueryResult {
            columns: column_names,
            rows: final_rows,
            affected_rows,
        })
    }

    /// Execute CREATE INDEX statement
    async fn execute_create_index(&mut self, stmt: CreateIndexStatement) -> Result<QueryResult> {
        // Get table schema
        let mut table_schema =
            self.context
                .catalog
                .get_table(&stmt.table_name)
                .ok_or_else(|| ShardForgeError::Query {
                    message: format!("Table '{}' does not exist", stmt.table_name),
                })?
                .clone();

        // Check if index already exists
        if table_schema.indexes.iter().any(|idx| idx.name == stmt.name) {
            if stmt.if_not_exists {
                return Ok(QueryResult {
                    columns: vec![],
                    rows: vec![],
                    affected_rows: 0,
                });
            } else {
                return Err(ShardForgeError::Query {
                    message: format!("Index '{}' already exists", stmt.name),
                });
            }
        }

        // Add index to schema
        table_schema.indexes.push(IndexSchema {
            name: stmt.name.clone(),
            table_name: stmt.table_name.clone(),
            columns: stmt.columns,
            index_type: stmt.index_type,
            unique: stmt.unique,
        });

        // Update catalog
        self.context.catalog.add_table(table_schema);

        // TODO: Build the index from existing data

        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: 1,
        })
    }

    /// Execute DROP INDEX statement
    async fn execute_drop_index(&mut self, stmt: DropIndexStatement) -> Result<QueryResult> {
        // Find the table that has this index
        let tables = self.context.catalog.list_tables();
        let mut found = false;

        for table_name in tables {
            if let Some(mut table_schema) = self.context.catalog.get_table(&table_name).cloned() {
                if let Some(index_pos) = table_schema.indexes.iter().position(|idx| idx.name == stmt.name) {
                    table_schema.indexes.remove(index_pos);
                    self.context.catalog.add_table(table_schema);
                    found = true;
                    break;
                }
            }
        }

        if !found && !stmt.if_exists {
            return Err(ShardForgeError::Query {
                message: format!("Index '{}' does not exist", stmt.name),
            });
        }

        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: if found { 1 } else { 0 },
        })
    }

    /// Execute ALTER TABLE statement
    async fn execute_alter_table(&mut self, stmt: AlterTableStatement) -> Result<QueryResult> {
        let mut table_schema =
            self.context
                .catalog
                .get_table(&stmt.name)
                .ok_or_else(|| ShardForgeError::Query {
                    message: format!("Table '{}' does not exist", stmt.name),
                })?
                .clone();

        match stmt.action {
            AlterTableAction::AddColumn(column_def) => {
                // Check if column already exists
                if table_schema.columns.iter().any(|c| c.name == column_def.name) {
                    return Err(ShardForgeError::Query {
                        message: format!("Column '{}' already exists", column_def.name),
                    });
                }

                table_schema.columns.push(ColumnSchema {
                    name: column_def.name,
                    data_type: column_def.data_type,
                    nullable: column_def.nullable,
                    default_value: column_def.default,
                    auto_increment: column_def.auto_increment,
                });
            }
            AlterTableAction::AddConstraint(_constraint) => {
                // TODO: Implement constraint addition
                return Err(ShardForgeError::Query {
                    message: "Adding constraints not yet implemented".to_string(),
                });
            }
            _ => {
                return Err(ShardForgeError::Query {
                    message: "ALTER TABLE action not yet implemented".to_string(),
                });
            }
        }

        // Update catalog
        self.context.catalog.add_table(table_schema);

        Ok(QueryResult {
            columns: vec![],
            rows: vec![],
            affected_rows: 1,
        })
    }

    /// Scan all keys for a table
    async fn scan_table_keys(&self, table_name: &str) -> Result<Vec<Key>> {
        let mut keys = Vec::new();
        let prefix = format!("{}:", table_name);

        // Use storage iterator to scan all keys with the table prefix
        // This is a simplified implementation - in production, we'd use proper iterator support
        for i in 0..1000 {
            // Arbitrary limit for now
            let key = Key::new(format!("{}{}",prefix, i).as_bytes());
            if self.context.storage.get(&key).await?.is_some() {
                keys.push(key);
            }
        }

        Ok(keys)
    }

    /// Evaluate a condition against a row
    fn evaluate_condition(
        &self,
        condition: &Expression,
        table_schema: &TableSchema,
        row_data: &[Value],
    ) -> Result<bool> {
        match condition {
            Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression_with_row(left, table_schema, row_data)?;
                let right_val = self.evaluate_expression_with_row(right, table_schema, row_data)?;

                match op {
                    BinaryOperator::Equal => Ok(left_val.as_ref() == right_val.as_ref()),
                    BinaryOperator::NotEqual => Ok(left_val.as_ref() != right_val.as_ref()),
                    BinaryOperator::LessThan => Ok(left_val.as_ref() < right_val.as_ref()),
                    BinaryOperator::LessThanOrEqual => Ok(left_val.as_ref() <= right_val.as_ref()),
                    BinaryOperator::GreaterThan => Ok(left_val.as_ref() > right_val.as_ref()),
                    BinaryOperator::GreaterThanOrEqual => {
                        Ok(left_val.as_ref() >= right_val.as_ref())
                    }
                    BinaryOperator::And => {
                        let left_bool = self.value_to_bool(&left_val)?;
                        let right_bool = self.value_to_bool(&right_val)?;
                        Ok(left_bool && right_bool)
                    }
                    BinaryOperator::Or => {
                        let left_bool = self.value_to_bool(&left_val)?;
                        let right_bool = self.value_to_bool(&right_val)?;
                        Ok(left_bool || right_bool)
                    }
                    _ => Err(ShardForgeError::Query {
                        message: format!("Operator {:?} not yet supported in WHERE clause", op),
                    }),
                }
            }
            Expression::IsNull { expr, negated } => {
                let val = self.evaluate_expression_with_row(expr, table_schema, row_data)?;
                let is_null = val.as_ref().is_empty();
                Ok(if *negated { !is_null } else { is_null })
            }
            _ => Err(ShardForgeError::Query {
                message: "Complex WHERE expressions not yet fully supported".to_string(),
            }),
        }
    }

    /// Evaluate an expression with access to row data
    fn evaluate_expression_with_row(
        &self,
        expr: &Expression,
        table_schema: &TableSchema,
        row_data: &[Value],
    ) -> Result<Value> {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Null => Ok(Value::new(b"")),
                Literal::Boolean(b) => Ok(Value::new(&[if *b { 1 } else { 0 }])),
                Literal::Integer(i) => Ok(Value::new(&i.to_le_bytes())),
                Literal::Float(f) => Ok(Value::new(&f.to_le_bytes())),
                Literal::String(s) => Ok(Value::new(s.as_bytes())),
                Literal::Binary(b) => Ok(Value::new(b)),
            },
            Expression::Column(col_name) => {
                let column_index =
                    table_schema
                        .columns
                        .iter()
                        .position(|c| c.name == *col_name)
                        .ok_or_else(|| ShardForgeError::Query {
                            message: format!("Column '{}' not found", col_name),
                        })?;

                if column_index < row_data.len() {
                    Ok(row_data[column_index].clone())
                } else {
                    Ok(Value::new(b""))
                }
            }
            Expression::BinaryOp { left, op, right } => {
                let left_val = self.evaluate_expression_with_row(left, table_schema, row_data)?;
                let right_val = self.evaluate_expression_with_row(right, table_schema, row_data)?;

                // Simplified arithmetic operations
                match op {
                    BinaryOperator::Add | BinaryOperator::Subtract | BinaryOperator::Multiply
                    | BinaryOperator::Divide => {
                        // For simplicity, treat as integer operations
                        if left_val.as_ref().len() == 8 && right_val.as_ref().len() == 8 {
                            let left_int = i64::from_le_bytes(
                                left_val.as_ref().try_into().unwrap_or([0; 8]),
                            );
                            let right_int = i64::from_le_bytes(
                                right_val.as_ref().try_into().unwrap_or([0; 8]),
                            );

                            let result = match op {
                                BinaryOperator::Add => left_int + right_int,
                                BinaryOperator::Subtract => left_int - right_int,
                                BinaryOperator::Multiply => left_int * right_int,
                                BinaryOperator::Divide => {
                                    if right_int == 0 {
                                        return Err(ShardForgeError::Query {
                                            message: "Division by zero".to_string(),
                                        });
                                    }
                                    left_int / right_int
                                }
                                _ => unreachable!(),
                            };

                            Ok(Value::new(&result.to_le_bytes()))
                        } else {
                            Err(ShardForgeError::Query {
                                message: "Arithmetic operations require integer operands".to_string(),
                            })
                        }
                    }
                    _ => Err(ShardForgeError::Query {
                        message: "Complex expressions not yet fully supported".to_string(),
                    }),
                }
            }
            _ => Err(ShardForgeError::Query {
                message: "Expression type not yet supported in execution".to_string(),
            }),
        }
    }

    /// Convert value to boolean
    fn value_to_bool(&self, value: &Value) -> Result<bool> {
        if value.as_ref().is_empty() {
            Ok(false)
        } else if value.as_ref().len() == 1 {
            Ok(value.as_ref()[0] != 0)
        } else {
            Ok(true)
        }
    }

    /// Evaluate an expression to a value (simplified)
    fn evaluate_expression(&self, expr: &Expression) -> Result<Value> {
        match expr {
            Expression::Literal(lit) => match lit {
                Literal::Null => Ok(Value::new(b"")),
                Literal::Boolean(b) => Ok(Value::new(&[if *b { 1 } else { 0 }])),
                Literal::Integer(i) => Ok(Value::new(&i.to_le_bytes())),
                Literal::Float(f) => Ok(Value::new(&f.to_le_bytes())),
                Literal::String(s) => Ok(Value::new(s.as_bytes())),
                Literal::Binary(b) => Ok(Value::new(b)),
            },
            _ => Err(ShardForgeError::Query {
                message: "Complex expressions not yet supported".to_string(),
            }),
        }
    }

    /// Extract key from WHERE clause (simplified for point queries)
    fn extract_key_from_where(&self, where_clause: &Expression, table_name: &str) -> Result<Key> {
        // For simplicity, assume WHERE clause is "id = value"
        match where_clause {
            Expression::BinaryOp {
                left: _left,
                op: BinaryOperator::Equal,
                right,
            } => {
                // Extract the value (simplified)
                if let Expression::Literal(Literal::Integer(id)) = right.as_ref() {
                    Ok(Key::new(format!("{}:{}", table_name, id).as_bytes()))
                } else {
                    Err(ShardForgeError::Query {
                        message: "Point queries currently support integer IDs only".to_string(),
                    })
                }
            }
            _ => Err(ShardForgeError::Query {
                message: "Point queries require equality condition".to_string(),
            }),
        }
    }

    /// Serialize row data (simplified)
    fn serialize_row(&self, row_data: &[Value]) -> Result<Vec<u8>> {
        // Simple serialization - concatenate lengths and data
        let mut result = Vec::new();
        result.extend_from_slice(&(row_data.len() as u32).to_le_bytes());

        for value in row_data {
            let data = value.as_ref();
            result.extend_from_slice(&(data.len() as u32).to_le_bytes());
            result.extend_from_slice(data);
        }

        Ok(result)
    }

    /// Deserialize row data (simplified)
    fn deserialize_row(&self, data: &[u8]) -> Result<Vec<Value>> {
        let mut offset = 0;
        let mut result = Vec::new();

        // Read number of columns
        if data.len() < 4 {
            return Err(ShardForgeError::Query {
                message: "Invalid row data format".to_string(),
            });
        }

        let column_count = u32::from_le_bytes([data[0], data[1], data[2], data[3]]) as usize;
        offset += 4;

        // Read each column value
        for _ in 0..column_count {
            if offset + 4 > data.len() {
                return Err(ShardForgeError::Query {
                    message: "Invalid row data format".to_string(),
                });
            }

            let value_len = u32::from_le_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            offset += 4;

            if offset + value_len > data.len() {
                return Err(ShardForgeError::Query {
                    message: "Invalid row data format".to_string(),
                });
            }

            let value_data = &data[offset..offset + value_len];
            result.push(Value::new(value_data));
            offset += value_len;
        }

        Ok(result)
    }

    /// Apply column selection
    fn apply_column_selection(
        &self,
        select_items: &[SelectItem],
        table_schema: &TableSchema,
        row_data: &[Value],
    ) -> Result<(Vec<String>, Vec<Value>)> {
        let mut selected_columns = Vec::new();
        let mut selected_data = Vec::new();

        for item in select_items {
            match item {
                SelectItem::Wildcard => {
                    // Select all columns
                    for (i, column) in table_schema.columns.iter().enumerate() {
                        selected_columns.push(column.name.clone());
                        if i < row_data.len() {
                            selected_data.push(row_data[i].clone());
                        } else {
                            selected_data.push(Value::new(b""));
                        }
                    }
                }
                SelectItem::Expression { expr, alias } => {
                    // For now, only support column references
                    if let Expression::Column(column_name) = expr {
                        let column_index = table_schema
                            .columns
                            .iter()
                            .position(|c| c.name == *column_name)
                            .ok_or_else(|| ShardForgeError::Query {
                                message: format!("Column '{}' not found", column_name),
                            })?;

                        let display_name = alias.as_ref().unwrap_or(column_name).clone();
                        selected_columns.push(display_name);

                        if column_index < row_data.len() {
                            selected_data.push(row_data[column_index].clone());
                        } else {
                            selected_data.push(Value::new(b""));
                        }
                    } else {
                        return Err(ShardForgeError::Query {
                            message: "Complex expressions in SELECT not yet supported".to_string(),
                        });
                    }
                }
            }
        }

        Ok((selected_columns, selected_data))
    }

    /// Get selected column names
    fn get_selected_column_names(
        &self,
        select_items: &[SelectItem],
        table_schema: &TableSchema,
    ) -> Result<Vec<String>> {
        let mut column_names = Vec::new();

        for item in select_items {
            match item {
                SelectItem::Wildcard => {
                    for column in &table_schema.columns {
                        column_names.push(column.name.clone());
                    }
                }
                SelectItem::Expression { expr, alias } => {
                    if let Expression::Column(column_name) = expr {
                        let display_name = alias.as_ref().unwrap_or(column_name).clone();
                        column_names.push(display_name);
                    } else {
                        return Err(ShardForgeError::Query {
                            message: "Complex expressions in SELECT not yet supported".to_string(),
                        });
                    }
                }
            }
        }

        Ok(column_names)
    }
}

impl Default for SchemaCatalog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shardforge_storage::MemoryEngine;

    #[tokio::test]
    async fn test_create_table() {
        let storage = Box::new(MemoryEngine::new(&Default::default()).await.unwrap());
        let mvcc = MVCCStorage::new();
        let catalog = SchemaCatalog::new();

        let context = ExecutionContext { storage, mvcc, catalog };

        let mut executor = QueryExecutor::new(context);

        let stmt = CreateTableStatement {
            if_not_exists: false,
            name: "users".to_string(),
            columns: vec![
                ColumnDef {
                    name: "id".to_string(),
                    data_type: DataType::Integer,
                    nullable: false,
                    default: None,
                    auto_increment: true,
                },
                ColumnDef {
                    name: "name".to_string(),
                    data_type: DataType::Varchar { length: Some(255) },
                    nullable: true,
                    default: None,
                    auto_increment: false,
                },
            ],
            constraints: vec![],
        };

        let result = executor.execute(Statement::CreateTable(stmt)).await.unwrap();
        assert_eq!(result.affected_rows, 1);

        // Check that table was added to catalog
        assert!(executor.context.catalog.get_table("users").is_some());
    }

    #[tokio::test]
    async fn test_insert_select() {
        let storage = Box::new(MemoryEngine::new(&Default::default()).await.unwrap());
        let mvcc = MVCCStorage::new();
        let mut catalog = SchemaCatalog::new();

        // Add table schema
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
                ColumnSchema {
                    name: "name".to_string(),
                    data_type: DataType::Varchar { length: Some(255) },
                    nullable: true,
                    default_value: None,
                    auto_increment: false,
                },
            ],
            primary_key: None,
            indexes: vec![],
        };
        catalog.add_table(table_schema);

        let context = ExecutionContext { storage, mvcc, catalog };

        let mut executor = QueryExecutor::new(context);

        // Insert a row
        let insert_stmt = InsertStatement {
            table_name: "users".to_string(),
            columns: None,
            values: vec![vec![
                Expression::Literal(Literal::Integer(1)),
                Expression::Literal(Literal::String("Alice".to_string())),
            ]],
        };

        let result = executor.execute(Statement::Insert(insert_stmt)).await.unwrap();
        assert_eq!(result.affected_rows, 1);

        // Select the row back
        let select_stmt = SelectStatement {
            distinct: false,
            columns: vec![SelectItem::Wildcard],
            from: Some("users".to_string()),
            where_clause: Some(Expression::BinaryOp {
                left: Box::new(Expression::Column("id".to_string())),
                op: BinaryOperator::Equal,
                right: Box::new(Expression::Literal(Literal::Integer(0))), // Row index 0
            }),
            group_by: vec![],
            having: None,
            order_by: vec![],
            limit: None,
            offset: None,
        };

        let result = executor.execute(Statement::Select(select_stmt)).await.unwrap();
        assert_eq!(result.affected_rows, 1);
        assert_eq!(result.columns.len(), 2);
        assert_eq!(result.rows.len(), 1);
    }
}
