//! Query execution engine

use std::collections::HashMap;

use shardforge_core::{Key, Result, ShardForgeError, Value};
use shardforge_storage::{StorageEngine, MVCCStorage};

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
        Self {
            tables: HashMap::new(),
        }
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
            Statement::Insert(stmt) => self.execute_insert(stmt).await,
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
        let columns = stmt.columns.into_iter().map(|col| ColumnSchema {
            name: col.name,
            data_type: col.data_type,
            nullable: col.nullable,
            default_value: col.default,
            auto_increment: col.auto_increment,
        }).collect();

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
        let table_schema = self.context.catalog.get_table(&stmt.table_name)
            .ok_or_else(|| ShardForgeError::Query {
                message: format!("Table '{}' does not exist", stmt.table_name),
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

    /// Execute SELECT statement (point queries only)
    async fn execute_select(&mut self, stmt: SelectStatement) -> Result<QueryResult> {
        // Get table name (simplified - single table queries only)
        let table_name = stmt.from.ok_or_else(|| ShardForgeError::Query {
            message: "FROM clause is required".to_string(),
        })?;

        // Get table schema
        let table_schema = self.context.catalog.get_table(&table_name)
            .ok_or_else(|| ShardForgeError::Query {
                message: format!("Table '{}' does not exist", table_name),
            })?;

        // For point queries, we need a WHERE clause with equality condition
        let where_clause = stmt.where_clause.ok_or_else(|| ShardForgeError::Query {
            message: "Point queries require WHERE clause with equality condition".to_string(),
        })?;

        // Extract key from WHERE clause (simplified)
        let key = self.extract_key_from_where(&where_clause, &table_name)?;

        // Fetch data from storage
        if let Some(storage_value) = self.context.storage.get(&key).await? {
            // Deserialize row data
            let row_data = self.deserialize_row(storage_value.as_ref())?;
            
            // Apply column selection
            let (selected_columns, selected_data) = self.apply_column_selection(&stmt.columns, table_schema, &row_data)?;

            Ok(QueryResult {
                columns: selected_columns,
                rows: vec![selected_data],
                affected_rows: 1,
            })
        } else {
            // No data found
            let selected_columns = self.get_selected_column_names(&stmt.columns, table_schema)?;
            Ok(QueryResult {
                columns: selected_columns,
                rows: vec![],
                affected_rows: 0,
            })
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
            Expression::BinaryOp { left, op: BinaryOperator::Equal, right } => {
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
                data[offset], data[offset + 1], data[offset + 2], data[offset + 3]
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
                        let column_index = table_schema.columns.iter()
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
        
        let context = ExecutionContext {
            storage,
            mvcc,
            catalog,
        };
        
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
        
        let context = ExecutionContext {
            storage,
            mvcc,
            catalog,
        };
        
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
