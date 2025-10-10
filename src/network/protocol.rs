//! Protocol buffer definitions and utilities

// Generated protobuf code will be included here
pub mod proto {
    tonic::include_proto!("shardforge.rpc");
}

pub use proto::*;

/// Convert from internal types to protobuf types
pub mod conversion {
    use super::proto;
    use shardforge_core::{Key, Value};

    /// Convert internal Value to protobuf Value
    pub fn value_to_proto(value: &Value) -> proto::Value {
        proto::Value {
            value: match value.as_ref() {
                b if b.is_empty() => Some(proto::value::Value::StringVal("".to_string())),
                b => Some(proto::value::Value::BytesVal(b.to_vec())),
            },
            is_null: false,
        }
    }

    /// Convert protobuf Value to internal Value
    pub fn value_from_proto(proto_value: &proto::Value) -> Value {
        if proto_value.is_null {
            return Value::new(b"");
        }

        match &proto_value.value {
            Some(proto::value::Value::StringVal(s)) => Value::new(s.as_bytes()),
            Some(proto::value::Value::BytesVal(b)) => Value::new(b),
            Some(proto::value::Value::BoolVal(b)) => Value::new(&[if *b { 1 } else { 0 }]),
            Some(proto::value::Value::Int32Val(i)) => Value::new(&i.to_le_bytes()),
            Some(proto::value::Value::Int64Val(i)) => Value::new(&i.to_le_bytes()),
            Some(proto::value::Value::FloatVal(f)) => Value::new(&f.to_le_bytes()),
            Some(proto::value::Value::DoubleVal(d)) => Value::new(&d.to_le_bytes()),
            _ => Value::new(b""),
        }
    }

    /// Convert internal Key to bytes
    pub fn key_to_bytes(key: &Key) -> Vec<u8> {
        key.as_ref().to_vec()
    }

    /// Convert bytes to internal Key
    pub fn key_from_bytes(bytes: &[u8]) -> Key {
        Key::new(bytes)
    }
}
