//! Abstract Syntax Tree for SQL statements

use serde::{Deserialize, Serialize};
// use std::collections::HashMap; // TODO: Remove when implementing table metadata

/// SQL Statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Statement {
    /// Data Definition Language
    CreateTable(CreateTableStatement),
    DropTable(DropTableStatement),
    AlterTable(AlterTableStatement),
    CreateIndex(CreateIndexStatement),
    DropIndex(DropIndexStatement),

    /// Data Manipulation Language
    Insert(InsertStatement),
    Update(UpdateStatement),
    Delete(DeleteStatement),
    Select(SelectStatement),

    /// Transaction Control
    Begin(BeginStatement),
    Commit(CommitStatement),
    Rollback(RollbackStatement),
}

/// CREATE TABLE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateTableStatement {
    pub if_not_exists: bool,
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub constraints: Vec<TableConstraint>,
}

/// DROP TABLE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropTableStatement {
    pub if_exists: bool,
    pub name: String,
    pub cascade: bool,
}

/// ALTER TABLE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AlterTableStatement {
    pub name: String,
    pub action: AlterTableAction,
}

/// ALTER TABLE actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterTableAction {
    AddColumn(ColumnDef),
    DropColumn {
        name: String,
        if_exists: bool,
    },
    AlterColumn {
        name: String,
        action: AlterColumnAction,
    },
    AddConstraint(TableConstraint),
    DropConstraint {
        name: String,
        if_exists: bool,
    },
}

/// ALTER COLUMN actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AlterColumnAction {
    SetDataType(DataType),
    SetDefault(Expression),
    DropDefault,
    SetNotNull,
    DropNotNull,
}

/// CREATE INDEX statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CreateIndexStatement {
    pub if_not_exists: bool,
    pub name: String,
    pub table_name: String,
    pub columns: Vec<String>,
    pub index_type: IndexType,
    pub unique: bool,
}

/// DROP INDEX statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DropIndexStatement {
    pub if_exists: bool,
    pub name: String,
}

/// INSERT statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InsertStatement {
    pub table_name: String,
    pub columns: Option<Vec<String>>,
    pub values: Vec<Vec<Expression>>,
}

/// UPDATE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UpdateStatement {
    pub table_name: String,
    pub assignments: Vec<Assignment>,
    pub where_clause: Option<Expression>,
}

/// DELETE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeleteStatement {
    pub table_name: String,
    pub where_clause: Option<Expression>,
}

/// SELECT statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SelectStatement {
    pub distinct: bool,
    pub columns: Vec<SelectItem>,
    pub from: Option<String>,
    pub where_clause: Option<Expression>,
    pub group_by: Vec<Expression>,
    pub having: Option<Expression>,
    pub order_by: Vec<OrderByItem>,
    pub limit: Option<u64>,
    pub offset: Option<u64>,
}

/// Transaction control statements
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BeginStatement {
    pub isolation_level: Option<IsolationLevel>,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CommitStatement;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RollbackStatement;

/// Column definition
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub data_type: DataType,
    pub nullable: bool,
    pub default: Option<Expression>,
    pub auto_increment: bool,
}

/// Data types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum DataType {
    Boolean,
    SmallInt,
    Integer,
    BigInt,
    Real,
    Double,
    Decimal { precision: u8, scale: u8 },
    Char { length: u32 },
    Varchar { length: Option<u32> },
    Text,
    Binary { length: u32 },
    Varbinary { length: Option<u32> },
    Blob,
    Date,
    Time,
    Timestamp,
    Json,
    Uuid,
}

/// Table constraints
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TableConstraint {
    PrimaryKey {
        columns: Vec<String>,
    },
    ForeignKey {
        columns: Vec<String>,
        referenced_table: String,
        referenced_columns: Vec<String>,
        on_delete: Option<ReferentialAction>,
        on_update: Option<ReferentialAction>,
    },
    Unique {
        columns: Vec<String>,
    },
    Check {
        expression: Expression,
    },
}

/// Referential actions
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ReferentialAction {
    Restrict,
    Cascade,
    SetNull,
    SetDefault,
    NoAction,
}

/// Index types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IndexType {
    BTree,
    Hash,
    GIN,
    GiST,
}

/// Assignment in UPDATE statement
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Assignment {
    pub column: String,
    pub value: Expression,
}

/// SELECT item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SelectItem {
    Wildcard,
    Expression {
        expr: Expression,
        alias: Option<String>,
    },
}

/// ORDER BY item
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OrderByItem {
    pub expression: Expression,
    pub direction: OrderDirection,
}

/// Order direction
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum OrderDirection {
    Asc,
    Desc,
}

/// Isolation levels
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum IsolationLevel {
    ReadUncommitted,
    ReadCommitted,
    RepeatableRead,
    Serializable,
}

/// SQL Expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Expression {
    /// Literal values
    Literal(Literal),
    /// Column reference
    Column(String),
    /// Binary operations
    BinaryOp {
        left: Box<Expression>,
        op: BinaryOperator,
        right: Box<Expression>,
    },
    /// Unary operations
    UnaryOp {
        op: UnaryOperator,
        expr: Box<Expression>,
    },
    /// Function calls
    Function { name: String, args: Vec<Expression> },
    /// CASE expression
    Case {
        when_clauses: Vec<WhenClause>,
        else_clause: Option<Box<Expression>>,
    },
    /// IN expression
    In {
        expr: Box<Expression>,
        list: Vec<Expression>,
        negated: bool,
    },
    /// BETWEEN expression
    Between {
        expr: Box<Expression>,
        low: Box<Expression>,
        high: Box<Expression>,
        negated: bool,
    },
    /// IS NULL expression
    IsNull {
        expr: Box<Expression>,
        negated: bool,
    },
}

/// Literal values
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Literal {
    Null,
    Boolean(bool),
    Integer(i64),
    Float(f64),
    String(String),
    Binary(Vec<u8>),
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum BinaryOperator {
    // Arithmetic
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,

    // Comparison
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,

    // Logical
    And,
    Or,

    // String
    Like,
    NotLike,
    ILike,
    NotILike,

    // Pattern matching
    Regex,
    NotRegex,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UnaryOperator {
    Plus,
    Minus,
    Not,
}

/// WHEN clause in CASE expression
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WhenClause {
    pub condition: Expression,
    pub result: Expression,
}

impl Statement {
    /// Get the type of statement for execution planning
    pub fn statement_type(&self) -> StatementType {
        match self {
            Statement::CreateTable(_)
            | Statement::DropTable(_)
            | Statement::AlterTable(_)
            | Statement::CreateIndex(_)
            | Statement::DropIndex(_) => StatementType::DDL,

            Statement::Insert(_) | Statement::Update(_) | Statement::Delete(_) => {
                StatementType::DML
            }

            Statement::Select(_) => StatementType::Query,

            Statement::Begin(_) | Statement::Commit(_) | Statement::Rollback(_) => {
                StatementType::Transaction
            }
        }
    }
}

/// Statement classification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum StatementType {
    DDL,         // Data Definition Language
    DML,         // Data Manipulation Language
    Query,       // SELECT statements
    Transaction, // Transaction control
}

impl DataType {
    /// Check if the data type is numeric
    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            DataType::SmallInt
                | DataType::Integer
                | DataType::BigInt
                | DataType::Real
                | DataType::Double
                | DataType::Decimal { .. }
        )
    }

    /// Check if the data type is string-like
    pub fn is_string(&self) -> bool {
        matches!(self, DataType::Char { .. } | DataType::Varchar { .. } | DataType::Text)
    }

    /// Check if the data type is binary-like
    pub fn is_binary(&self) -> bool {
        matches!(self, DataType::Binary { .. } | DataType::Varbinary { .. } | DataType::Blob)
    }

    /// Check if the data type is temporal
    pub fn is_temporal(&self) -> bool {
        matches!(self, DataType::Date | DataType::Time | DataType::Timestamp)
    }
}
