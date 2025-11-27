//! Import and export utilities for CSV and JSON

use shardforge_core::{Result, ShardForgeError, Value};
use std::fs::File;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::path::Path;

use shardforge::sql::{
    ast::*, executor::QueryResult, executor::SchemaCatalog, parser::SqlParser,
};

/// CSV import options
#[derive(Debug, Clone)]
pub struct CsvImportOptions {
    /// Has header row
    pub has_header: bool,
    /// Field delimiter
    pub delimiter: u8,
    /// Quote character
    pub quote: u8,
    /// Skip N rows
    pub skip_rows: usize,
    /// Null value representation
    pub null_value: String,
}

impl Default for CsvImportOptions {
    fn default() -> Self {
        Self {
            has_header: true,
            delimiter: b',',
            quote: b'"',
            skip_rows: 0,
            null_value: String::new(),
        }
    }
}

/// CSV export options
#[derive(Debug, Clone)]
pub struct CsvExportOptions {
    /// Include header row
    pub include_header: bool,
    /// Field delimiter
    pub delimiter: u8,
    /// Quote character
    pub quote: u8,
    /// Null value representation
    pub null_value: String,
}

impl Default for CsvExportOptions {
    fn default() -> Self {
        Self {
            include_header: true,
            delimiter: b',',
            quote: b'"',
            null_value: "NULL".to_string(),
        }
    }
}

/// JSON export options
#[derive(Debug, Clone)]
pub struct JsonExportOptions {
    /// Pretty print JSON
    pub pretty: bool,
    /// Array format (vs. newline-delimited JSON)
    pub array_format: bool,
}

impl Default for JsonExportOptions {
    fn default() -> Self {
        Self {
            pretty: false,
            array_format: true,
        }
    }
}

/// CSV importer
pub struct CsvImporter {
    options: CsvImportOptions,
}

impl CsvImporter {
    /// Create a new CSV importer
    pub fn new(options: CsvImportOptions) -> Self {
        Self { options }
    }

    /// Import CSV data into a table
    pub fn import_from_file<P: AsRef<Path>>(
        &self,
        file_path: P,
        table_name: &str,
        column_types: &[(String, DataType)],
    ) -> Result<Vec<InsertStatement>> {
        let file = File::open(file_path).map_err(|e| ShardForgeError::Io {
            message: format!("Failed to open CSV file: {}", e),
        })?;

        let reader = BufReader::new(file);
        let mut statements = Vec::new();
        let mut lines = reader.lines();

        // Skip header if present
        if self.options.has_header {
            lines.next();
        }

        // Skip specified rows
        for _ in 0..self.options.skip_rows {
            lines.next();
        }

        // Process each line
        for (line_num, line_result) in lines.enumerate() {
            let line = line_result.map_err(|e| ShardForgeError::Io {
                message: format!("Failed to read line {}: {}", line_num, e),
            })?;

            let values = self.parse_csv_line(&line, column_types)?;

            let stmt = InsertStatement {
                table_name: table_name.to_string(),
                columns: None,
                values: vec![values],
            };

            statements.push(stmt);
        }

        Ok(statements)
    }

    /// Parse a CSV line into values
    fn parse_csv_line(
        &self,
        line: &str,
        column_types: &[(String, DataType)],
    ) -> Result<Vec<Expression>> {
        let fields = self.split_csv_line(line);

        if fields.len() != column_types.len() {
            return Err(ShardForgeError::Parse {
                message: format!(
                    "Expected {} columns, found {}",
                    column_types.len(),
                    fields.len()
                ),
            });
        }

        let mut values = Vec::new();

        for (field, (_col_name, col_type)) in fields.iter().zip(column_types.iter()) {
            let value = self.parse_field_value(field, col_type)?;
            values.push(Expression::Literal(value));
        }

        Ok(values)
    }

    /// Split CSV line into fields
    fn split_csv_line(&self, line: &str) -> Vec<String> {
        let mut fields = Vec::new();
        let mut current_field = String::new();
        let mut in_quotes = false;
        let delimiter_char = self.options.delimiter as char;
        let quote_char = self.options.quote as char;

        for ch in line.chars() {
            if ch == quote_char {
                in_quotes = !in_quotes;
            } else if ch == delimiter_char && !in_quotes {
                fields.push(current_field.trim().to_string());
                current_field.clear();
            } else {
                current_field.push(ch);
            }
        }

        fields.push(current_field.trim().to_string());
        fields
    }

    /// Parse a field value based on column type
    fn parse_field_value(&self, field: &str, col_type: &DataType) -> Result<Literal> {
        // Handle NULL values
        if field == self.options.null_value || field.is_empty() {
            return Ok(Literal::Null);
        }

        match col_type {
            DataType::Integer | DataType::SmallInt | DataType::BigInt => {
                let value = field.parse::<i64>().map_err(|_| ShardForgeError::Parse {
                    message: format!("Invalid integer value: {}", field),
                })?;
                Ok(Literal::Integer(value))
            }
            DataType::Real | DataType::Double => {
                let value = field.parse::<f64>().map_err(|_| ShardForgeError::Parse {
                    message: format!("Invalid float value: {}", field),
                })?;
                Ok(Literal::Float(value))
            }
            DataType::Boolean => {
                let value = match field.to_lowercase().as_str() {
                    "true" | "t" | "yes" | "y" | "1" => true,
                    "false" | "f" | "no" | "n" | "0" => false,
                    _ => {
                        return Err(ShardForgeError::Parse {
                            message: format!("Invalid boolean value: {}", field),
                        })
                    }
                };
                Ok(Literal::Boolean(value))
            }
            _ => Ok(Literal::String(field.to_string())),
        }
    }
}

/// CSV exporter
pub struct CsvExporter {
    options: CsvExportOptions,
}

impl CsvExporter {
    /// Create a new CSV exporter
    pub fn new(options: CsvExportOptions) -> Self {
        Self { options }
    }

    /// Export query results to CSV file
    pub fn export_to_file<P: AsRef<Path>>(
        &self,
        result: &QueryResult,
        file_path: P,
    ) -> Result<()> {
        let file = File::create(file_path).map_err(|e| ShardForgeError::Io {
            message: format!("Failed to create CSV file: {}", e),
        })?;

        let mut writer = BufWriter::new(file);

        // Write header if requested
        if self.options.include_header {
            self.write_header(&mut writer, &result.columns)?;
        }

        // Write rows
        for row in &result.rows {
            self.write_row(&mut writer, row)?;
        }

        writer.flush().map_err(|e| ShardForgeError::Io {
            message: format!("Failed to flush CSV file: {}", e),
        })?;

        Ok(())
    }

    /// Write header row
    fn write_header<W: Write>(&self, writer: &mut W, columns: &[String]) -> Result<()> {
        let header = columns.join(&(self.options.delimiter as char).to_string());
        writeln!(writer, "{}", header).map_err(|e| ShardForgeError::Io {
            message: format!("Failed to write CSV header: {}", e),
        })?;
        Ok(())
    }

    /// Write data row
    fn write_row<W: Write>(&self, writer: &mut W, row: &[Value]) -> Result<()> {
        let fields: Vec<String> = row
            .iter()
            .map(|value| self.format_field_value(value))
            .collect();

        let line = fields.join(&(self.options.delimiter as char).to_string());
        writeln!(writer, "{}", line).map_err(|e| ShardForgeError::Io {
            message: format!("Failed to write CSV row: {}", e),
        })?;

        Ok(())
    }

    /// Format a field value for CSV output
    fn format_field_value(&self, value: &Value) -> String {
        let bytes = value.as_ref();

        if bytes.is_empty() {
            return self.options.null_value.clone();
        }

        // Try to convert to string
        if let Ok(s) = std::str::from_utf8(bytes) {
            // Quote if contains delimiter or newline
            if s.contains(self.options.delimiter as char) || s.contains('\n') {
                format!("{}{}{}", self.options.quote as char, s, self.options.quote as char)
            } else {
                s.to_string()
            }
        } else {
            // Binary data - hex encode
            format!("0x{}", hex::encode(bytes))
        }
    }
}

/// JSON exporter
pub struct JsonExporter {
    options: JsonExportOptions,
}

impl JsonExporter {
    /// Create a new JSON exporter
    pub fn new(options: JsonExportOptions) -> Self {
        Self { options }
    }

    /// Export query results to JSON file
    pub fn export_to_file<P: AsRef<Path>>(
        &self,
        result: &QueryResult,
        file_path: P,
    ) -> Result<()> {
        let file = File::create(file_path).map_err(|e| ShardForgeError::Io {
            message: format!("Failed to create JSON file: {}", e),
        })?;

        let mut writer = BufWriter::new(file);

        if self.options.array_format {
            self.export_array_format(&mut writer, result)?;
        } else {
            self.export_ndjson_format(&mut writer, result)?;
        }

        writer.flush().map_err(|e| ShardForgeError::Io {
            message: format!("Failed to flush JSON file: {}", e),
        })?;

        Ok(())
    }

    /// Export as JSON array
    fn export_array_format<W: Write>(&self, writer: &mut W, result: &QueryResult) -> Result<()> {
        let mut records = Vec::new();

        for row in &result.rows {
            let mut record = serde_json::Map::new();

            for (i, col_name) in result.columns.iter().enumerate() {
                if i < row.len() {
                    let value = self.value_to_json(&row[i]);
                    record.insert(col_name.clone(), value);
                }
            }

            records.push(serde_json::Value::Object(record));
        }

        let json = serde_json::Value::Array(records);

        if self.options.pretty {
            serde_json::to_writer_pretty(writer, &json)
        } else {
            serde_json::to_writer(writer, &json)
        }
        .map_err(|e| ShardForgeError::Io {
            message: format!("Failed to write JSON: {}", e),
        })?;

        Ok(())
    }

    /// Export as newline-delimited JSON
    fn export_ndjson_format<W: Write>(&self, writer: &mut W, result: &QueryResult) -> Result<()> {
        for row in &result.rows {
            let mut record = serde_json::Map::new();

            for (i, col_name) in result.columns.iter().enumerate() {
                if i < row.len() {
                    let value = self.value_to_json(&row[i]);
                    record.insert(col_name.clone(), value);
                }
            }

            let json = serde_json::Value::Object(record);
            serde_json::to_writer(&mut *writer, &json).map_err(|e| ShardForgeError::Io {
                message: format!("Failed to write JSON record: {}", e),
            })?;
            writeln!(writer).map_err(|e| ShardForgeError::Io {
                message: format!("Failed to write newline: {}", e),
            })?;
        }

        Ok(())
    }

    /// Convert Value to JSON value
    fn value_to_json(&self, value: &Value) -> serde_json::Value {
        let bytes = value.as_ref();

        if bytes.is_empty() {
            return serde_json::Value::Null;
        }

        // Try integer (8 bytes)
        if bytes.len() == 8 {
            if let Ok(int_bytes) = bytes.try_into() {
                let int_val = i64::from_le_bytes(int_bytes);
                return serde_json::Value::Number(int_val.into());
            }
        }

        // Try float (8 bytes)
        if bytes.len() == 8 {
            if let Ok(float_bytes) = bytes.try_into() {
                let float_val = f64::from_le_bytes(float_bytes);
                if let Some(num) = serde_json::Number::from_f64(float_val) {
                    return serde_json::Value::Number(num);
                }
            }
        }

        // Try boolean (1 byte)
        if bytes.len() == 1 {
            return serde_json::Value::Bool(bytes[0] != 0);
        }

        // Try string
        if let Ok(s) = std::str::from_utf8(bytes) {
            return serde_json::Value::String(s.to_string());
        }

        // Binary data - base64 encode
        serde_json::Value::String(base64::encode(bytes))
    }
}

/// JSON importer
pub struct JsonImporter;

impl JsonImporter {
    /// Import JSON array to INSERT statements
    pub fn import_from_file<P: AsRef<Path>>(
        file_path: P,
        table_name: &str,
        column_types: &[(String, DataType)],
    ) -> Result<Vec<InsertStatement>> {
        let file = File::open(file_path).map_err(|e| ShardForgeError::Io {
            message: format!("Failed to open JSON file: {}", e),
        })?;

        let json_value: serde_json::Value = serde_json::from_reader(file).map_err(|e| {
            ShardForgeError::Parse {
                message: format!("Failed to parse JSON: {}", e),
            }
        })?;

        let mut statements = Vec::new();

        match json_value {
            serde_json::Value::Array(records) => {
                for record in records {
                    let values = Self::json_object_to_values(&record, column_types)?;
                    statements.push(InsertStatement {
                        table_name: table_name.to_string(),
                        columns: None,
                        values: vec![values],
                    });
                }
            }
            serde_json::Value::Object(_) => {
                // Single object - treat as one record
                let values = Self::json_object_to_values(&json_value, column_types)?;
                statements.push(InsertStatement {
                    table_name: table_name.to_string(),
                    columns: None,
                    values: vec![values],
                });
            }
            _ => {
                return Err(ShardForgeError::Parse {
                    message: "JSON must be an array or object".to_string(),
                })
            }
        }

        Ok(statements)
    }

    /// Convert JSON object to SQL values
    fn json_object_to_values(
        obj: &serde_json::Value,
        column_types: &[(String, DataType)],
    ) -> Result<Vec<Expression>> {
        let mut values = Vec::new();

        let obj_map = obj.as_object().ok_or_else(|| ShardForgeError::Parse {
            message: "Expected JSON object".to_string(),
        })?;

        for (col_name, col_type) in column_types {
            let json_value = obj_map.get(col_name).unwrap_or(&serde_json::Value::Null);

            let literal = Self::json_value_to_literal(json_value, col_type)?;
            values.push(Expression::Literal(literal));
        }

        Ok(values)
    }

    /// Convert JSON value to SQL literal
    fn json_value_to_literal(value: &serde_json::Value, col_type: &DataType) -> Result<Literal> {
        match value {
            serde_json::Value::Null => Ok(Literal::Null),
            serde_json::Value::Bool(b) => Ok(Literal::Boolean(*b)),
            serde_json::Value::Number(n) => {
                if col_type.is_numeric() {
                    if let Some(i) = n.as_i64() {
                        Ok(Literal::Integer(i))
                    } else if let Some(f) = n.as_f64() {
                        Ok(Literal::Float(f))
                    } else {
                        Err(ShardForgeError::Parse {
                            message: "Invalid numeric value".to_string(),
                        })
                    }
                } else {
                    Ok(Literal::String(n.to_string()))
                }
            }
            serde_json::Value::String(s) => Ok(Literal::String(s.clone())),
            _ => Err(ShardForgeError::Parse {
                message: format!("Unsupported JSON value type"),
            }),
        }
    }
}

// Add hex and base64 crates to dependencies
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

mod base64 {
    pub fn encode(bytes: &[u8]) -> String {
        use std::fmt::Write;
        let mut result = String::new();
        for chunk in bytes.chunks(3) {
            let mut buf = [0u8; 3];
            buf[..chunk.len()].copy_from_slice(chunk);
            let _ = write!(&mut result, "{:02x}{:02x}{:02x}", buf[0], buf[1], buf[2]);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_csv_field_parsing() {
        let importer = CsvImporter::new(CsvImportOptions::default());

        let line = "1,John,30";
        let column_types = vec![
            ("id".to_string(), DataType::Integer),
            ("name".to_string(), DataType::Varchar { length: None }),
            ("age".to_string(), DataType::Integer),
        ];

        let values = importer.parse_csv_line(line, &column_types).unwrap();
        assert_eq!(values.len(), 3);
    }

    #[test]
    fn test_csv_field_splitting() {
        let importer = CsvImporter::new(CsvImportOptions::default());

        let line = r#"1,"John Doe",30"#;
        let fields = importer.split_csv_line(line);

        assert_eq!(fields.len(), 3);
        assert_eq!(fields[0], "1");
        assert_eq!(fields[1], "John Doe");
        assert_eq!(fields[2], "30");
    }
}

