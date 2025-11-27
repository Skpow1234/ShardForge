//! Comprehensive CLI integration tests

use shardforge_cli::*;
use shardforge_core::Value;
use std::fs;
use std::io::Write;
use tempfile::NamedTempFile;

#[test]
fn test_format_query_results() {
    let columns = vec!["id".to_string(), "name".to_string(), "age".to_string()];
    let rows = vec![
        vec![
            Value::new(b"1"),
            Value::new(b"Alice"),
            Value::new(&30i64.to_le_bytes()),
        ],
        vec![
            Value::new(b"2"),
            Value::new(b"Bob"),
            Value::new(&25i64.to_le_bytes()),
        ],
    ];

    let table = format_query_results(&columns, &rows);
    
    // Check that table contains headers
    assert!(table.contains("id"));
    assert!(table.contains("name"));
    assert!(table.contains("age"));
}

#[test]
fn test_csv_import_basic() {
    use shardforge::sql::ast::DataType;

    // Create temporary CSV file
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "id,name,age").unwrap();
    writeln!(temp_file, "1,Alice,30").unwrap();
    writeln!(temp_file, "2,Bob,25").unwrap();
    temp_file.flush().unwrap();

    let column_types = vec![
        ("id".to_string(), DataType::Integer),
        ("name".to_string(), DataType::Varchar { length: None }),
        ("age".to_string(), DataType::Integer),
    ];

    let options = CsvImportOptions::default();
    let importer = CsvImporter::new(options);

    let statements = importer
        .import_from_file(temp_file.path(), "users", &column_types)
        .unwrap();

    assert_eq!(statements.len(), 2);
    assert_eq!(statements[0].table_name, "users");
    assert_eq!(statements[0].values[0].len(), 3);
}

#[test]
fn test_csv_import_no_header() {
    use shardforge::sql::ast::DataType;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "1,Alice,30").unwrap();
    writeln!(temp_file, "2,Bob,25").unwrap();
    temp_file.flush().unwrap();

    let column_types = vec![
        ("id".to_string(), DataType::Integer),
        ("name".to_string(), DataType::Varchar { length: None }),
        ("age".to_string(), DataType::Integer),
    ];

    let options = CsvImportOptions {
        has_header: false,
        ..Default::default()
    };
    let importer = CsvImporter::new(options);

    let statements = importer
        .import_from_file(temp_file.path(), "users", &column_types)
        .unwrap();

    assert_eq!(statements.len(), 2);
}

#[test]
fn test_csv_import_custom_delimiter() {
    use shardforge::sql::ast::DataType;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "id;name;age").unwrap();
    writeln!(temp_file, "1;Alice;30").unwrap();
    temp_file.flush().unwrap();

    let column_types = vec![
        ("id".to_string(), DataType::Integer),
        ("name".to_string(), DataType::Varchar { length: None }),
        ("age".to_string(), DataType::Integer),
    ];

    let options = CsvImportOptions {
        delimiter: b';',
        ..Default::default()
    };
    let importer = CsvImporter::new(options);

    let statements = importer
        .import_from_file(temp_file.path(), "users", &column_types)
        .unwrap();

    assert_eq!(statements.len(), 1);
}

#[test]
fn test_csv_export_basic() {
    use shardforge::sql::executor::QueryResult;

    let result = QueryResult {
        columns: vec!["id".to_string(), "name".to_string()],
        rows: vec![
            vec![Value::new(b"1"), Value::new(b"Alice")],
            vec![Value::new(b"2"), Value::new(b"Bob")],
        ],
        affected_rows: 2,
    };

    let temp_file = NamedTempFile::new().unwrap();
    let options = CsvExportOptions::default();
    let exporter = CsvExporter::new(options);

    exporter.export_to_file(&result, temp_file.path()).unwrap();

    let content = fs::read_to_string(temp_file.path()).unwrap();
    
    assert!(content.contains("id,name"));
    assert!(content.contains("1,Alice"));
    assert!(content.contains("2,Bob"));
}

#[test]
fn test_csv_export_no_header() {
    use shardforge::sql::executor::QueryResult;

    let result = QueryResult {
        columns: vec!["id".to_string(), "name".to_string()],
        rows: vec![vec![Value::new(b"1"), Value::new(b"Alice")]],
        affected_rows: 1,
    };

    let temp_file = NamedTempFile::new().unwrap();
    let options = CsvExportOptions {
        include_header: false,
        ..Default::default()
    };
    let exporter = CsvExporter::new(options);

    exporter.export_to_file(&result, temp_file.path()).unwrap();

    let content = fs::read_to_string(temp_file.path()).unwrap();
    
    assert!(!content.contains("id,name"));
    assert!(content.contains("1,Alice"));
}

#[test]
fn test_json_export_array_format() {
    use shardforge::sql::executor::QueryResult;

    let result = QueryResult {
        columns: vec!["id".to_string(), "name".to_string()],
        rows: vec![
            vec![Value::new(&1i64.to_le_bytes()), Value::new(b"Alice")],
            vec![Value::new(&2i64.to_le_bytes()), Value::new(b"Bob")],
        ],
        affected_rows: 2,
    };

    let temp_file = NamedTempFile::new().unwrap();
    let options = JsonExportOptions {
        array_format: true,
        pretty: false,
    };
    let exporter = JsonExporter::new(options);

    exporter.export_to_file(&result, temp_file.path()).unwrap();

    let content = fs::read_to_string(temp_file.path()).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();

    assert!(json.is_array());
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[test]
fn test_json_export_pretty() {
    use shardforge::sql::executor::QueryResult;

    let result = QueryResult {
        columns: vec!["id".to_string()],
        rows: vec![vec![Value::new(&1i64.to_le_bytes())]],
        affected_rows: 1,
    };

    let temp_file = NamedTempFile::new().unwrap();
    let options = JsonExportOptions {
        array_format: true,
        pretty: true,
    };
    let exporter = JsonExporter::new(options);

    exporter.export_to_file(&result, temp_file.path()).unwrap();

    let content = fs::read_to_string(temp_file.path()).unwrap();
    
    // Pretty format should have newlines and indentation
    assert!(content.contains('\n'));
    assert!(content.contains("  "));
}

#[test]
fn test_json_import_array() {
    use shardforge::sql::ast::DataType;

    let json_data = r#"[
        {"id": 1, "name": "Alice", "age": 30},
        {"id": 2, "name": "Bob", "age": 25}
    ]"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", json_data).unwrap();
    temp_file.flush().unwrap();

    let column_types = vec![
        ("id".to_string(), DataType::Integer),
        ("name".to_string(), DataType::Varchar { length: None }),
        ("age".to_string(), DataType::Integer),
    ];

    let statements = JsonImporter::import_from_file(temp_file.path(), "users", &column_types).unwrap();

    assert_eq!(statements.len(), 2);
    assert_eq!(statements[0].table_name, "users");
}

#[test]
fn test_json_import_single_object() {
    use shardforge::sql::ast::DataType;

    let json_data = r#"{"id": 1, "name": "Alice"}"#;

    let mut temp_file = NamedTempFile::new().unwrap();
    write!(temp_file, "{}", json_data).unwrap();
    temp_file.flush().unwrap();

    let column_types = vec![
        ("id".to_string(), DataType::Integer),
        ("name".to_string(), DataType::Varchar { length: None }),
    ];

    let statements = JsonImporter::import_from_file(temp_file.path(), "users", &column_types).unwrap();

    assert_eq!(statements.len(), 1);
}

#[test]
fn test_execute_sql_file() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "-- Comment line").unwrap();
    writeln!(temp_file, "SELECT * FROM users;").unwrap();
    writeln!(temp_file, "").unwrap();
    writeln!(temp_file, "INSERT INTO users VALUES (1, 'Alice');").unwrap();
    temp_file.flush().unwrap();

    let statements = execute_sql_file(temp_file.path()).unwrap();

    assert_eq!(statements.len(), 2);
    assert!(statements[0].contains("SELECT"));
    assert!(statements[1].contains("INSERT"));
}

#[test]
fn test_execute_sql_file_multiline() {
    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "SELECT *").unwrap();
    writeln!(temp_file, "FROM users").unwrap();
    writeln!(temp_file, "WHERE age > 18;").unwrap();
    temp_file.flush().unwrap();

    let statements = execute_sql_file(temp_file.path()).unwrap();

    assert_eq!(statements.len(), 1);
    assert!(statements[0].contains("SELECT"));
    assert!(statements[0].contains("FROM"));
    assert!(statements[0].contains("WHERE"));
}

#[test]
fn test_csv_quoted_fields() {
    use shardforge::sql::ast::DataType;

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "id,name,description").unwrap();
    writeln!(temp_file, r#"1,"John Doe","A person with a, comma""#).unwrap();
    temp_file.flush().unwrap();

    let column_types = vec![
        ("id".to_string(), DataType::Integer),
        ("name".to_string(), DataType::Varchar { length: None }),
        ("description".to_string(), DataType::Text),
    ];

    let options = CsvImportOptions::default();
    let importer = CsvImporter::new(options);

    let statements = importer
        .import_from_file(temp_file.path(), "users", &column_types)
        .unwrap();

    assert_eq!(statements.len(), 1);
}

#[test]
fn test_csv_null_values() {
    use shardforge::sql::ast::{DataType, Literal};

    let mut temp_file = NamedTempFile::new().unwrap();
    writeln!(temp_file, "id,name,age").unwrap();
    writeln!(temp_file, "1,Alice,").unwrap(); // Empty age field
    temp_file.flush().unwrap();

    let column_types = vec![
        ("id".to_string(), DataType::Integer),
        ("name".to_string(), DataType::Varchar { length: None }),
        ("age".to_string(), DataType::Integer),
    ];

    let options = CsvImportOptions::default();
    let importer = CsvImporter::new(options);

    let statements = importer
        .import_from_file(temp_file.path(), "users", &column_types)
        .unwrap();

    assert_eq!(statements.len(), 1);
    // Age field should be NULL
    if let shardforge::sql::ast::Expression::Literal(lit) = &statements[0].values[0][2] {
        assert!(matches!(lit, Literal::Null));
    }
}

