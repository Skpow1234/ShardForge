//! Prepared statement support

use std::collections::HashMap;
use std::sync::Arc;

use shardforge_core::{Result, ShardForgeError};

use crate::sql::ast::*;
use crate::sql::parser::SqlParser;

/// Prepared statement identifier
pub type PreparedStatementId = String;

/// Prepared statement
#[derive(Debug, Clone)]
pub struct PreparedStatement {
    /// Statement ID
    pub id: PreparedStatementId,
    /// Original SQL text
    pub sql: String,
    /// Parsed statement
    pub statement: Statement,
    /// Parameter placeholders
    pub parameters: Vec<ParameterInfo>,
    /// Creation timestamp
    pub created_at: std::time::SystemTime,
    /// Number of times executed
    pub execution_count: u64,
}

/// Parameter information
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// Parameter index (1-based)
    pub index: u32,
    /// Parameter name (if named)
    pub name: Option<String>,
    /// Expected data type (if known)
    pub data_type: Option<DataType>,
}

/// Prepared statement manager
pub struct PreparedStatementManager {
    /// Prepared statements by ID
    statements: HashMap<PreparedStatementId, PreparedStatement>,
    /// SQL parser
    parser: SqlParser,
}

impl PreparedStatementManager {
    /// Create a new prepared statement manager
    pub fn new() -> Self {
        Self {
            statements: HashMap::new(),
            parser: SqlParser::new(),
        }
    }

    /// Prepare a SQL statement
    pub fn prepare(&mut self, id: PreparedStatementId, sql: String) -> Result<PreparedStatementId> {
        // Check if statement with this ID already exists
        if self.statements.contains_key(&id) {
            return Err(ShardForgeError::Query {
                message: format!("Prepared statement '{}' already exists", id),
            });
        }

        // Parse the SQL
        let statement = self.parser.parse(&sql)?;

        // Extract parameters (simplified - would need proper parameter parsing)
        let parameters = Self::extract_parameters(&sql);

        // Create prepared statement
        let prepared = PreparedStatement {
            id: id.clone(),
            sql,
            statement,
            parameters,
            created_at: std::time::SystemTime::now(),
            execution_count: 0,
        };

        // Store prepared statement
        self.statements.insert(id.clone(), prepared);

        Ok(id)
    }

    /// Get a prepared statement by ID
    pub fn get(&self, id: &PreparedStatementId) -> Option<&PreparedStatement> {
        self.statements.get(id)
    }

    /// Get a mutable reference to a prepared statement
    pub fn get_mut(&mut self, id: &PreparedStatementId) -> Option<&mut PreparedStatement> {
        self.statements.get_mut(id)
    }

    /// Execute a prepared statement with parameters
    pub fn bind_parameters(
        &mut self,
        id: &PreparedStatementId,
        parameters: Vec<Literal>,
    ) -> Result<Statement> {
        let prepared = self.statements.get_mut(id).ok_or_else(|| {
            ShardForgeError::Query {
                message: format!("Prepared statement '{}' not found", id),
            }
        })?;

        // Validate parameter count
        if parameters.len() != prepared.parameters.len() {
            return Err(ShardForgeError::Query {
                message: format!(
                    "Expected {} parameters, got {}",
                    prepared.parameters.len(),
                    parameters.len()
                ),
            });
        }

        // Increment execution count
        prepared.execution_count += 1;

        // Clone the statement and bind parameters
        let mut bound_statement = prepared.statement.clone();
        Self::replace_parameters(&mut bound_statement, &parameters)?;

        Ok(bound_statement)
    }

    /// Deallocate a prepared statement
    pub fn deallocate(&mut self, id: &PreparedStatementId) -> Result<()> {
        if self.statements.remove(id).is_none() {
            return Err(ShardForgeError::Query {
                message: format!("Prepared statement '{}' not found", id),
            });
        }
        Ok(())
    }

    /// List all prepared statements
    pub fn list(&self) -> Vec<&PreparedStatement> {
        self.statements.values().collect()
    }

    /// Extract parameter placeholders from SQL
    fn extract_parameters(sql: &str) -> Vec<ParameterInfo> {
        let mut parameters = Vec::new();
        let mut index = 1;

        // Simple regex-like extraction of $1, $2, etc.
        for (i, c) in sql.chars().enumerate() {
            if c == '$' {
                if let Some(next_char) = sql.chars().nth(i + 1) {
                    if next_char.is_ascii_digit() {
                        parameters.push(ParameterInfo {
                            index,
                            name: None,
                            data_type: None,
                        });
                        index += 1;
                    }
                }
            }
        }

        parameters
    }

    /// Replace parameter placeholders with actual values
    fn replace_parameters(statement: &mut Statement, parameters: &[Literal]) -> Result<()> {
        // This is a simplified implementation
        // In a real implementation, we would traverse the AST and replace parameter nodes
        
        match statement {
            Statement::Insert(ref mut insert) => {
                for row in &mut insert.values {
                    for expr in row {
                        Self::replace_expression_parameters(expr, parameters)?;
                    }
                }
            }
            Statement::Update(ref mut update) => {
                for assignment in &mut update.assignments {
                    Self::replace_expression_parameters(&mut assignment.value, parameters)?;
                }
                if let Some(ref mut where_clause) = update.where_clause {
                    Self::replace_expression_parameters(where_clause, parameters)?;
                }
            }
            Statement::Delete(ref mut delete) => {
                if let Some(ref mut where_clause) = delete.where_clause {
                    Self::replace_expression_parameters(where_clause, parameters)?;
                }
            }
            Statement::Select(ref mut select) => {
                if let Some(ref mut where_clause) = select.where_clause {
                    Self::replace_expression_parameters(where_clause, parameters)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    /// Replace parameters in an expression (simplified)
    fn replace_expression_parameters(_expr: &mut Expression, _parameters: &[Literal]) -> Result<()> {
        // This would need to traverse the expression tree and replace parameter nodes
        // For now, this is a placeholder
        Ok(())
    }
}

impl Default for PreparedStatementManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prepare_statement() {
        let mut manager = PreparedStatementManager::new();
        
        let sql = "SELECT * FROM users WHERE id = $1".to_string();
        let result = manager.prepare("stmt1".to_string(), sql);
        
        assert!(result.is_ok());
        assert!(manager.get(&"stmt1".to_string()).is_some());
    }

    #[test]
    fn test_parameter_extraction() {
        let sql = "SELECT * FROM users WHERE id = $1 AND name = $2";
        let params = PreparedStatementManager::extract_parameters(sql);
        
        assert_eq!(params.len(), 2);
        assert_eq!(params[0].index, 1);
        assert_eq!(params[1].index, 2);
    }

    #[test]
    fn test_deallocate() {
        let mut manager = PreparedStatementManager::new();
        
        let sql = "SELECT * FROM users".to_string();
        manager.prepare("stmt1".to_string(), sql).unwrap();
        
        assert!(manager.get(&"stmt1".to_string()).is_some());
        
        manager.deallocate(&"stmt1".to_string()).unwrap();
        assert!(manager.get(&"stmt1".to_string()).is_none());
    }
}

