//! Comprehensive SQL integration tests

use shardforge::sql::*;
use shardforge_core::*;
use shardforge_storage::{MemoryEngine, MVCCStorage};

/// Helper function to create a test execution context
async fn create_test_context() -> ExecutionContext {
    let storage = Box::new(MemoryEngine::new(&Default::default()).await.unwrap());
    let mvcc = MVCCStorage::new();
    let catalog = SchemaCatalog::new();

    ExecutionContext {
        storage,
        mvcc,
        catalog,
    }
}

#[tokio::test]
async fn test_create_and_drop_table() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create table
    let sql = "CREATE TABLE users (id INTEGER NOT NULL, name VARCHAR(255), age INTEGER)";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 1);

    // Verify table exists
    assert!(executor.context.catalog.get_table("users").is_some());

    // Drop table
    let sql = "DROP TABLE users";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 1);

    // Verify table is gone
    assert!(executor.context.catalog.get_table("users").is_none());
}

#[tokio::test]
async fn test_create_table_if_not_exists() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create table
    let sql = "CREATE TABLE IF NOT EXISTS users (id INTEGER)";
    let stmt = parser.parse(sql).unwrap();
    executor.execute(stmt).await.unwrap();

    // Try to create again - should not error
    let sql = "CREATE TABLE IF NOT EXISTS users (id INTEGER)";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 0);
}

#[tokio::test]
async fn test_insert_and_select() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create table
    let sql = "CREATE TABLE users (id INTEGER, name VARCHAR(255), age INTEGER)";
    let stmt = parser.parse(sql).unwrap();
    executor.execute(stmt).await.unwrap();

    // Insert data
    let sql = "INSERT INTO users VALUES (1, 'Alice', 30)";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 1);

    // Insert more data
    let sql = "INSERT INTO users VALUES (2, 'Bob', 25), (3, 'Charlie', 35)";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 2);

    // Select all
    let sql = "SELECT * FROM users";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.rows.len(), 3);
    assert_eq!(result.columns.len(), 3);
}

#[tokio::test]
async fn test_select_with_where_clause() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE users (id INTEGER, name VARCHAR(255), age INTEGER)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO users VALUES (1, 'Alice', 30), (2, 'Bob', 25), (3, 'Charlie', 35)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Select with WHERE clause
    let sql = "SELECT * FROM users WHERE age > 28";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    
    // Should return Alice (30) and Charlie (35)
    assert_eq!(result.rows.len(), 2);
}

#[tokio::test]
async fn test_update_statement() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE users (id INTEGER, name VARCHAR(255), age INTEGER)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO users VALUES (1, 'Alice', 30), (2, 'Bob', 25)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Update with WHERE clause
    let sql = "UPDATE users SET age = 31 WHERE id = 1";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 1);

    // Update without WHERE clause (updates all)
    let sql = "UPDATE users SET name = 'Unknown'";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 2);
}

#[tokio::test]
async fn test_delete_statement() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE users (id INTEGER, name VARCHAR(255), age INTEGER)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO users VALUES (1, 'Alice', 30), (2, 'Bob', 25), (3, 'Charlie', 35)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Delete with WHERE clause
    let sql = "DELETE FROM users WHERE age < 30";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 1); // Should delete Bob

    // Verify remaining data
    let sql = "SELECT * FROM users";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 2); // Alice and Charlie remain
}

#[tokio::test]
async fn test_select_with_order_by() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE users (id INTEGER, name VARCHAR(255), age INTEGER)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO users VALUES (3, 'Charlie', 35), (1, 'Alice', 30), (2, 'Bob', 25)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Select with ORDER BY
    let sql = "SELECT * FROM users ORDER BY age ASC";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    
    assert_eq!(result.rows.len(), 3);
    // Results should be ordered by age: Bob(25), Alice(30), Charlie(35)
}

#[tokio::test]
async fn test_select_with_limit_offset() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE users (id INTEGER, name VARCHAR(255))";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO users VALUES (1, 'Alice'), (2, 'Bob'), (3, 'Charlie'), (4, 'David'), (5, 'Eve')";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Select with LIMIT
    let sql = "SELECT * FROM users LIMIT 3";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 3);

    // Select with LIMIT and OFFSET
    let sql = "SELECT * FROM users LIMIT 2 OFFSET 2";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 2);
}

#[tokio::test]
async fn test_select_distinct() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE orders (id INTEGER, status VARCHAR(255))";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO orders VALUES (1, 'pending'), (2, 'completed'), (3, 'pending'), (4, 'completed')";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Select DISTINCT
    let sql = "SELECT DISTINCT status FROM orders";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    
    // Should return only 2 distinct values: pending and completed
    assert_eq!(result.rows.len(), 2);
}

#[tokio::test]
async fn test_aggregate_functions() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE sales (id INTEGER, amount INTEGER, region VARCHAR(255))";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO sales VALUES (1, 100, 'North'), (2, 200, 'South'), (3, 150, 'North')";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Test COUNT
    let sql = "SELECT COUNT(id) FROM sales";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 1);

    // Test SUM
    let sql = "SELECT SUM(amount) FROM sales";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 1);

    // Test AVG
    let sql = "SELECT AVG(amount) FROM sales";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 1);

    // Test MIN and MAX
    let sql = "SELECT MIN(amount), MAX(amount) FROM sales";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 1);
    assert_eq!(result.columns.len(), 2);
}

#[tokio::test]
async fn test_group_by() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE sales (id INTEGER, amount INTEGER, region VARCHAR(255))";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO sales VALUES (1, 100, 'North'), (2, 200, 'South'), (3, 150, 'North'), (4, 300, 'South')";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Group by region with aggregates
    let sql = "SELECT region, COUNT(id), SUM(amount) FROM sales GROUP BY region";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    
    // Should return 2 groups: North and South
    assert_eq!(result.rows.len(), 2);
    assert_eq!(result.columns.len(), 3);
}

#[tokio::test]
async fn test_create_and_drop_index() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create table
    let sql = "CREATE TABLE users (id INTEGER, email VARCHAR(255))";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Create index
    let sql = "CREATE INDEX idx_email ON users (email)";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 1);

    // Verify index exists
    let table_schema = executor.context.catalog.get_table("users").unwrap();
    assert_eq!(table_schema.indexes.len(), 1);
    assert_eq!(table_schema.indexes[0].name, "idx_email");

    // Drop index
    let sql = "DROP INDEX idx_email";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 1);
}

#[tokio::test]
async fn test_alter_table_add_column() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create table
    let sql = "CREATE TABLE users (id INTEGER, name VARCHAR(255))";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Alter table to add column
    let sql = "ALTER TABLE users ADD COLUMN email VARCHAR(255)";
    let stmt = parser.parse(sql).unwrap();
    let result = executor.execute(stmt).await.unwrap();
    assert_eq!(result.affected_rows, 1);

    // Verify column was added
    let table_schema = executor.context.catalog.get_table("users").unwrap();
    assert_eq!(table_schema.columns.len(), 3);
    assert_eq!(table_schema.columns[2].name, "email");
}

#[tokio::test]
async fn test_complex_where_conditions() {
    let context = create_test_context().await;
    let mut executor = QueryExecutor::new(context);
    let mut parser = SqlParser::new();

    // Create and populate table
    let sql = "CREATE TABLE users (id INTEGER, age INTEGER, active INTEGER)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    let sql = "INSERT INTO users VALUES (1, 25, 1), (2, 30, 0), (3, 35, 1), (4, 28, 1)";
    executor.execute(parser.parse(sql).unwrap()).await.unwrap();

    // Complex WHERE with AND
    let sql = "SELECT * FROM users WHERE age > 25 AND active = 1";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 2); // id 3 and 4

    // Complex WHERE with OR
    let sql = "SELECT * FROM users WHERE age < 27 OR age > 33";
    let result = executor.execute(parser.parse(sql).unwrap()).await.unwrap();
    assert_eq!(result.rows.len(), 2); // id 1 and 3
}

#[test]
fn test_query_cache() {
    let mut cache = QueryCache::new();
    
    let key1 = CacheKey::new("SELECT * FROM users", b"");
    let result1 = QueryResult {
        columns: vec!["id".to_string(), "name".to_string()],
        rows: vec![],
        affected_rows: 0,
    };

    // Put and get
    cache.put(key1.clone(), result1.clone()).unwrap();
    let cached = cache.get(&key1);
    assert!(cached.is_some());

    // Test cache miss
    let key2 = CacheKey::new("SELECT * FROM orders", b"");
    let cached = cache.get(&key2);
    assert!(cached.is_none());

    // Test statistics
    assert_eq!(cache.statistics().hits, 1);
    assert_eq!(cache.statistics().misses, 1);
    assert_eq!(cache.hit_rate(), 0.5);
}

#[test]
fn test_prepared_statements() {
    let mut manager = PreparedStatementManager::new();
    
    // Prepare statement
    let sql = "SELECT * FROM users WHERE id = $1".to_string();
    let id = manager.prepare("stmt1".to_string(), sql).unwrap();
    assert_eq!(id, "stmt1");

    // Verify statement exists
    let prepared = manager.get(&id);
    assert!(prepared.is_some());
    assert_eq!(prepared.unwrap().parameters.len(), 1);

    // Deallocate
    manager.deallocate(&id).unwrap();
    assert!(manager.get(&id).is_none());
}

#[test]
fn test_query_optimizer() {
    let optimizer = QueryOptimizer::new();
    
    let logical_plan = LogicalPlan::TableScan {
        table_name: "users".to_string(),
        projection: None,
        filter: None,
    };

    let physical_plan = optimizer.optimize(logical_plan).unwrap();
    
    match physical_plan {
        PhysicalPlan::SeqScan { .. } => {
            // Expected
        }
        _ => panic!("Expected SeqScan plan"),
    }

    // Test cost estimation
    let cost = optimizer.estimate_cost(&physical_plan);
    assert!(cost.total_cost > 0.0);
}

#[test]
fn test_statistics_collector() {
    let mut collector = StatisticsCollector::new();
    
    // Create table statistics
    let mut stats = TableStatistics::new("users".to_string());
    stats.set_row_count(1000);
    
    let mut col_stats = ColumnStatistics::new("status".to_string());
    col_stats.set_distinct_count(5);
    stats.add_column_stats(col_stats);
    
    collector.update_table_stats(stats);

    // Test selectivity estimation
    let selectivity = collector.estimate_selectivity("users", "status", "=", b"active");
    assert_eq!(selectivity, 0.2); // 1/5 distinct values

    // Test row count estimation
    let estimated_rows = collector.estimate_row_count("users", selectivity);
    assert_eq!(estimated_rows, 200); // 1000 * 0.2
}

