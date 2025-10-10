//! SQL parser implementation

use std::collections::HashMap;

use shardforge_core::{Result, ShardForgeError};

use crate::sql::ast::*;

/// SQL Parser
pub struct SqlParser {
    tokens: Vec<Token>,
    position: usize,
}

/// SQL Token
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // Keywords
    Create,
    Table,
    Drop,
    Alter,
    Index,
    Insert,
    Into,
    Update,
    Delete,
    Select,
    From,
    Where,
    Group,
    By,
    Having,
    Order,
    Limit,
    Offset,
    Distinct,
    Begin,
    Commit,
    Rollback,
    If,
    Not,
    Exists,
    Cascade,
    Restrict,
    Add,
    Column,
    Constraint,
    Primary,
    Key,
    On,
    Foreign,
    References,
    Unique,
    Check,
    Default,
    Null,
    Auto,
    Increment,
    
    // Data types
    Boolean,
    SmallInt,
    Integer,
    BigInt,
    Real,
    Double,
    Decimal,
    Char,
    Varchar,
    Text,
    Binary,
    Varbinary,
    Blob,
    Date,
    Time,
    Timestamp,
    Json,
    Uuid,
    
    // Operators
    Equals,
    NotEquals,
    LessThan,
    LessThanOrEquals,
    GreaterThan,
    GreaterThanOrEquals,
    Plus,
    Minus,
    Multiply,
    Divide,
    Modulo,
    And,
    Or,
    Like,
    In,
    Between,
    Is,
    
    // Literals
    StringLit(String),
    IntegerLit(i64),
    FloatLit(f64),
    BooleanLit(bool),
    
    // Identifiers
    Identifier(String),
    
    // Punctuation
    LeftParen,
    RightParen,
    Comma,
    Semicolon,
    Dot,
    
    // Special
    Wildcard, // *
    EOF,
}

/// Lexical analysis errors
#[derive(Debug, thiserror::Error)]
pub enum LexError {
    #[error("Unexpected character: {0}")]
    UnexpectedChar(char),
    #[error("Unterminated string literal")]
    UnterminatedString,
    #[error("Invalid number format")]
    InvalidNumber,
}

/// Parse errors
#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("Unexpected token: {0:?}")]
    UnexpectedToken(Token),
    #[error("Expected token: {0:?}, found: {1:?}")]
    ExpectedToken(Token, Token),
    #[error("Unexpected end of input")]
    UnexpectedEOF,
    #[error("Unsupported feature: {0}")]
    UnsupportedFeature(String),
}

impl SqlParser {
    /// Create a new SQL parser
    pub fn new() -> Self {
        Self {
            tokens: Vec::new(),
            position: 0,
        }
    }

    /// Parse a SQL statement from a string
    pub fn parse(&mut self, sql: &str) -> Result<Statement> {
        // Tokenize the input
        self.tokens = self.tokenize(sql)?;
        self.position = 0;

        // Parse the statement
        self.parse_statement()
    }

    /// Tokenize SQL input
    fn tokenize(&self, sql: &str) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        let mut chars = sql.chars().peekable();

        while let Some(&ch) = chars.peek() {
            match ch {
                ' ' | '\t' | '\n' | '\r' => {
                    chars.next();
                }
                '(' => {
                    tokens.push(Token::LeftParen);
                    chars.next();
                }
                ')' => {
                    tokens.push(Token::RightParen);
                    chars.next();
                }
                ',' => {
                    tokens.push(Token::Comma);
                    chars.next();
                }
                ';' => {
                    tokens.push(Token::Semicolon);
                    chars.next();
                }
                '.' => {
                    tokens.push(Token::Dot);
                    chars.next();
                }
                '*' => {
                    tokens.push(Token::Wildcard);
                    chars.next();
                }
                '+' => {
                    tokens.push(Token::Plus);
                    chars.next();
                }
                '-' => {
                    chars.next();
                    if chars.peek() == Some(&'-') {
                        // Skip line comment
                        chars.next();
                        while let Some(&c) = chars.peek() {
                            chars.next();
                            if c == '\n' {
                                break;
                            }
                        }
                    } else {
                        tokens.push(Token::Minus);
                    }
                }
                '/' => {
                    tokens.push(Token::Divide);
                    chars.next();
                }
                '%' => {
                    tokens.push(Token::Modulo);
                    chars.next();
                }
                '=' => {
                    tokens.push(Token::Equals);
                    chars.next();
                }
                '<' => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(Token::LessThanOrEquals);
                    } else if chars.peek() == Some(&'>') {
                        chars.next();
                        tokens.push(Token::NotEquals);
                    } else {
                        tokens.push(Token::LessThan);
                    }
                }
                '>' => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(Token::GreaterThanOrEquals);
                    } else {
                        tokens.push(Token::GreaterThan);
                    }
                }
                '!' => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        tokens.push(Token::NotEquals);
                    } else {
                        return Err(ShardForgeError::Parse {
                            message: format!("Unexpected character: {}", ch),
                        });
                    }
                }
                '\'' => {
                    chars.next();
                    let mut string_val = String::new();
                    let mut escaped = false;
                    
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if escaped {
                            match c {
                                '\'' => string_val.push('\''),
                                '\\' => string_val.push('\\'),
                                'n' => string_val.push('\n'),
                                't' => string_val.push('\t'),
                                'r' => string_val.push('\r'),
                                _ => {
                                    string_val.push('\\');
                                    string_val.push(c);
                                }
                            }
                            escaped = false;
                        } else if c == '\\' {
                            escaped = true;
                        } else if c == '\'' {
                            break;
                        } else {
                            string_val.push(c);
                        }
                    }
                    
                    tokens.push(Token::StringLit(string_val));
                }
                '"' => {
                    chars.next();
                    let mut identifier = String::new();
                    
                    while let Some(&c) = chars.peek() {
                        chars.next();
                        if c == '"' {
                            break;
                        }
                        identifier.push(c);
                    }
                    
                    tokens.push(Token::Identifier(identifier));
                }
                _ if ch.is_alphabetic() || ch == '_' => {
                    let mut identifier = String::new();
                    
                    while let Some(&c) = chars.peek() {
                        if c.is_alphanumeric() || c == '_' {
                            identifier.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    
                    let token = self.keyword_or_identifier(identifier);
                    tokens.push(token);
                }
                _ if ch.is_numeric() => {
                    let mut number = String::new();
                    let mut is_float = false;
                    
                    while let Some(&c) = chars.peek() {
                        if c.is_numeric() {
                            number.push(c);
                            chars.next();
                        } else if c == '.' && !is_float {
                            is_float = true;
                            number.push(c);
                            chars.next();
                        } else {
                            break;
                        }
                    }
                    
                    if is_float {
                        let float_val: f64 = number.parse().map_err(|_| ShardForgeError::Parse {
                            message: "Invalid float literal".to_string(),
                        })?;
                        tokens.push(Token::FloatLit(float_val));
                    } else {
                        let int_val: i64 = number.parse().map_err(|_| ShardForgeError::Parse {
                            message: "Invalid integer literal".to_string(),
                        })?;
                        tokens.push(Token::IntegerLit(int_val));
                    }
                }
                _ => {
                    return Err(ShardForgeError::Parse {
                        message: format!("Unexpected character: {}", ch),
                    });
                }
            }
        }

        tokens.push(Token::EOF);
        Ok(tokens)
    }

    /// Convert string to keyword or identifier token
    fn keyword_or_identifier(&self, s: String) -> Token {
        match s.to_uppercase().as_str() {
            "CREATE" => Token::Create,
            "TABLE" => Token::Table,
            "DROP" => Token::Drop,
            "ALTER" => Token::Alter,
            "INDEX" => Token::Index,
            "INSERT" => Token::Insert,
            "INTO" => Token::Into,
            "UPDATE" => Token::Update,
            "DELETE" => Token::Delete,
            "SELECT" => Token::Select,
            "FROM" => Token::From,
            "WHERE" => Token::Where,
            "GROUP" => Token::Group,
            "BY" => Token::By,
            "HAVING" => Token::Having,
            "ORDER" => Token::Order,
            "LIMIT" => Token::Limit,
            "OFFSET" => Token::Offset,
            "DISTINCT" => Token::Distinct,
            "BEGIN" => Token::Begin,
            "COMMIT" => Token::Commit,
            "ROLLBACK" => Token::Rollback,
            "IF" => Token::If,
            "NOT" => Token::Not,
            "EXISTS" => Token::Exists,
            "CASCADE" => Token::Cascade,
            "RESTRICT" => Token::Restrict,
            "ADD" => Token::Add,
            "COLUMN" => Token::Column,
            "CONSTRAINT" => Token::Constraint,
            "PRIMARY" => Token::Primary,
            "KEY" => Token::Key,
            "FOREIGN" => Token::Foreign,
            "REFERENCES" => Token::References,
            "UNIQUE" => Token::Unique,
            "CHECK" => Token::Check,
            "DEFAULT" => Token::Default,
            "NULL" => Token::Null,
            "AUTO" => Token::Auto,
            "INCREMENT" => Token::Increment,
            "BOOLEAN" => Token::Boolean,
            "SMALLINT" => Token::SmallInt,
            "INTEGER" | "INT" => Token::Integer,
            "BIGINT" => Token::BigInt,
            "REAL" => Token::Real,
            "DOUBLE" => Token::Double,
            "DECIMAL" | "NUMERIC" => Token::Decimal,
            "CHAR" => Token::Char,
            "VARCHAR" => Token::Varchar,
            "TEXT" => Token::Text,
            "BINARY" => Token::Binary,
            "VARBINARY" => Token::Varbinary,
            "BLOB" => Token::Blob,
            "DATE" => Token::Date,
            "TIME" => Token::Time,
            "TIMESTAMP" => Token::Timestamp,
            "JSON" => Token::Json,
            "UUID" => Token::Uuid,
            "AND" => Token::And,
            "OR" => Token::Or,
            "LIKE" => Token::Like,
            "IN" => Token::In,
            "BETWEEN" => Token::Between,
            "IS" => Token::Is,
            "TRUE" => Token::BooleanLit(true),
            "FALSE" => Token::BooleanLit(false),
            _ => Token::Identifier(s),
        }
    }

    /// Parse a statement
    fn parse_statement(&mut self) -> Result<Statement> {
        match self.current_token() {
            Token::Create => self.parse_create_statement(),
            Token::Drop => self.parse_drop_statement(),
            Token::Alter => self.parse_alter_statement(),
            Token::Insert => self.parse_insert_statement(),
            Token::Update => self.parse_update_statement(),
            Token::Delete => self.parse_delete_statement(),
            Token::Select => self.parse_select_statement(),
            Token::Begin => self.parse_begin_statement(),
            Token::Commit => self.parse_commit_statement(),
            Token::Rollback => self.parse_rollback_statement(),
            token => Err(ShardForgeError::Parse {
                message: format!("Unexpected token at start of statement: {:?}", token),
            }),
        }
    }

    /// Parse CREATE statement
    fn parse_create_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Create)?;
        
        match self.current_token() {
            Token::Table => self.parse_create_table_statement(),
            Token::Index => self.parse_create_index_statement(),
            token => Err(ShardForgeError::Parse {
                message: format!("Expected TABLE or INDEX after CREATE, found: {:?}", token),
            }),
        }
    }

    /// Parse CREATE TABLE statement
    fn parse_create_table_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Table)?;
        
        let if_not_exists = if self.current_token() == &Token::If {
            self.consume_token(Token::If)?;
            self.consume_token(Token::Not)?;
            self.consume_token(Token::Exists)?;
            true
        } else {
            false
        };

        let name = self.parse_identifier()?;
        
        self.consume_token(Token::LeftParen)?;
        
        let mut columns = Vec::new();
        let mut constraints = Vec::new();
        
        loop {
            if self.current_token() == &Token::RightParen {
                break;
            }
            
            if self.current_token() == &Token::Constraint {
                constraints.push(self.parse_table_constraint()?);
            } else {
                columns.push(self.parse_column_def()?);
            }
            
            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        self.consume_token(Token::RightParen)?;

        Ok(Statement::CreateTable(CreateTableStatement {
            if_not_exists,
            name,
            columns,
            constraints,
        }))
    }

    /// Parse CREATE INDEX statement
    fn parse_create_index_statement(&mut self) -> Result<Statement> {
        self.consume_token(Token::Index)?;
        
        let if_not_exists = if self.current_token() == &Token::If {
            self.consume_token(Token::If)?;
            self.consume_token(Token::Not)?;
            self.consume_token(Token::Exists)?;
            true
        } else {
            false
        };

        let name = self.parse_identifier()?;
        self.consume_token(Token::On)?;
        let table_name = self.parse_identifier()?;
        
        self.consume_token(Token::LeftParen)?;
        let mut columns = Vec::new();
        
        loop {
            columns.push(self.parse_identifier()?);
            
            if self.current_token() == &Token::Comma {
                self.advance();
            } else {
                break;
            }
        }
        
        self.consume_token(Token::RightParen)?;

        Ok(Statement::CreateIndex(CreateIndexStatement {
            if_not_exists,
            name,
            table_name,
            columns,
            index_type: IndexType::BTree, // Default to B-tree
            unique: false, // TODO: Parse UNIQUE keyword
        }))
    }

    /// Parse column definition
    fn parse_column_def(&mut self) -> Result<ColumnDef> {
        let name = self.parse_identifier()?;
        let data_type = self.parse_data_type()?;
        
        let mut nullable = true;
        let mut default = None;
        let mut auto_increment = false;
        
        // Parse column constraints
        while self.position < self.tokens.len() {
            match self.current_token() {
                Token::Not => {
                    self.advance();
                    self.consume_token(Token::Null)?;
                    nullable = false;
                }
                Token::Default => {
                    self.advance();
                    default = Some(self.parse_expression()?);
                }
                Token::Auto => {
                    self.advance();
                    self.consume_token(Token::Increment)?;
                    auto_increment = true;
                }
                _ => break,
            }
        }

        Ok(ColumnDef {
            name,
            data_type,
            nullable,
            default,
            auto_increment,
        })
    }

    /// Parse data type
    fn parse_data_type(&mut self) -> Result<DataType> {
        match self.current_token() {
            Token::Boolean => {
                self.advance();
                Ok(DataType::Boolean)
            }
            Token::SmallInt => {
                self.advance();
                Ok(DataType::SmallInt)
            }
            Token::Integer => {
                self.advance();
                Ok(DataType::Integer)
            }
            Token::BigInt => {
                self.advance();
                Ok(DataType::BigInt)
            }
            Token::Real => {
                self.advance();
                Ok(DataType::Real)
            }
            Token::Double => {
                self.advance();
                Ok(DataType::Double)
            }
            Token::Varchar => {
                self.advance();
                if self.current_token() == &Token::LeftParen {
                    self.advance();
                    let length = self.parse_integer_literal()?;
                    self.consume_token(Token::RightParen)?;
                    Ok(DataType::Varchar { length: Some(length as u32) })
                } else {
                    Ok(DataType::Varchar { length: None })
                }
            }
            Token::Text => {
                self.advance();
                Ok(DataType::Text)
            }
            token => Err(ShardForgeError::Parse {
                message: format!("Expected data type, found: {:?}", token),
            }),
        }
    }

    /// Helper methods
    fn current_token(&self) -> &Token {
        self.tokens.get(self.position).unwrap_or(&Token::EOF)
    }
    
    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }
    
    fn consume_token(&mut self, expected: Token) -> Result<()> {
        if self.current_token() == &expected {
            self.advance();
            Ok(())
        } else {
            Err(ShardForgeError::Parse {
                message: format!("Expected {:?}, found {:?}", expected, self.current_token()),
            })
        }
    }
    
    fn parse_identifier(&mut self) -> Result<String> {
        match self.current_token() {
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(name)
            }
            token => Err(ShardForgeError::Parse {
                message: format!("Expected identifier, found: {:?}", token),
            }),
        }
    }
    
    fn parse_integer_literal(&mut self) -> Result<i64> {
        match self.current_token() {
            Token::IntegerLit(value) => {
                let value = *value;
                self.advance();
                Ok(value)
            }
            token => Err(ShardForgeError::Parse {
                message: format!("Expected integer literal, found: {:?}", token),
            }),
        }
    }

    // Placeholder implementations for other parsing methods
    fn parse_drop_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "DROP statements not yet implemented".to_string(),
        })
    }

    fn parse_alter_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "ALTER statements not yet implemented".to_string(),
        })
    }

    fn parse_insert_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "INSERT statements not yet implemented".to_string(),
        })
    }

    fn parse_update_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "UPDATE statements not yet implemented".to_string(),
        })
    }

    fn parse_delete_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "DELETE statements not yet implemented".to_string(),
        })
    }

    fn parse_select_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "SELECT statements not yet implemented".to_string(),
        })
    }

    fn parse_begin_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "BEGIN statements not yet implemented".to_string(),
        })
    }

    fn parse_commit_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "COMMIT statements not yet implemented".to_string(),
        })
    }

    fn parse_rollback_statement(&mut self) -> Result<Statement> {
        Err(ShardForgeError::Parse {
            message: "ROLLBACK statements not yet implemented".to_string(),
        })
    }

    fn parse_table_constraint(&mut self) -> Result<TableConstraint> {
        Err(ShardForgeError::Parse {
            message: "Table constraints not yet implemented".to_string(),
        })
    }

    fn parse_expression(&mut self) -> Result<Expression> {
        match self.current_token() {
            Token::StringLit(s) => {
                let s = s.clone();
                self.advance();
                Ok(Expression::Literal(Literal::String(s)))
            }
            Token::IntegerLit(i) => {
                let i = *i;
                self.advance();
                Ok(Expression::Literal(Literal::Integer(i)))
            }
            Token::FloatLit(f) => {
                let f = *f;
                self.advance();
                Ok(Expression::Literal(Literal::Float(f)))
            }
            Token::BooleanLit(b) => {
                let b = *b;
                self.advance();
                Ok(Expression::Literal(Literal::Boolean(b)))
            }
            Token::Null => {
                self.advance();
                Ok(Expression::Literal(Literal::Null))
            }
            Token::Identifier(name) => {
                let name = name.clone();
                self.advance();
                Ok(Expression::Column(name))
            }
            token => Err(ShardForgeError::Parse {
                message: format!("Unexpected token in expression: {:?}", token),
            }),
        }
    }
}

impl Default for SqlParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_table_basic() {
        let mut parser = SqlParser::new();
        let sql = "CREATE TABLE users (id INTEGER, name VARCHAR(255))";
        
        let result = parser.parse(sql).unwrap();
        
        match result {
            Statement::CreateTable(stmt) => {
                assert_eq!(stmt.name, "users");
                assert_eq!(stmt.columns.len(), 2);
                assert_eq!(stmt.columns[0].name, "id");
                assert_eq!(stmt.columns[1].name, "name");
            }
            _ => panic!("Expected CreateTable statement"),
        }
    }

    #[test]
    fn test_create_table_if_not_exists() {
        let mut parser = SqlParser::new();
        let sql = "CREATE TABLE IF NOT EXISTS users (id INTEGER)";
        
        let result = parser.parse(sql).unwrap();
        
        match result {
            Statement::CreateTable(stmt) => {
                assert!(stmt.if_not_exists);
                assert_eq!(stmt.name, "users");
            }
            _ => panic!("Expected CreateTable statement"),
        }
    }

    #[test]
    fn test_tokenizer() {
        let parser = SqlParser::new();
        let tokens = parser.tokenize("CREATE TABLE test").unwrap();
        
        assert_eq!(tokens[0], Token::Create);
        assert_eq!(tokens[1], Token::Table);
        assert_eq!(tokens[2], Token::Identifier("test".to_string()));
        assert_eq!(tokens[3], Token::EOF);
    }
}
