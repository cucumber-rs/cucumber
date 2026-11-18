//! Materialize untrusted JSON according to its lexical token types.
//!
//! With serde_json's arbitrary_precision feature, Value's map visitor recognizes
//! a private number marker. A user-supplied object with that key must still be an
//! object, never a number. RawValue lets us choose typed deserialization from the
//! actual source token without losing exact numeric spellings.

use std::collections::BTreeMap;
use serde::de::Error as _;
use serde_json::{Map, Number, Value, value::RawValue};

const MAX_NESTING: usize = 128;

pub(super) fn parse(text: &str) -> Result<Value, serde_json::Error> {
    let raw: Box<RawValue> = serde_json::from_str(text)?;
    materialize(&raw, 0)
}

fn materialize(raw: &RawValue, depth: usize) -> Result<Value, serde_json::Error> {
    let text = raw.get().trim();
    let token = text.as_bytes().first().copied();
    if matches!(token, Some(b'{') | Some(b'[')) && depth >= MAX_NESTING {
        return Err(serde_json::Error::custom("JSON nesting exceeds 128 levels"));
    }
    match token {
        Some(b'{') => {
            let members: BTreeMap<String, Box<RawValue>> = serde_json::from_str(text)?;
            let mut object = Map::new();
            for (key, value) in members {
                object.insert(key, materialize(&value, depth + 1)?);
            }
            Ok(Value::Object(object))
        }
        Some(b'[') => {
            let elements: Vec<Box<RawValue>> = serde_json::from_str(text)?;
            let values = elements.iter().map(|value| materialize(value, depth + 1))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(Value::Array(values))
        }
        Some(b'"') => serde_json::from_str::<String>(text).map(Value::String),
        Some(b't') | Some(b'f') => serde_json::from_str::<bool>(text).map(Value::Bool),
        Some(b'n') => serde_json::from_str::<()>(text).map(|()| Value::Null),
        Some(b'-' | b'0'..=b'9') => serde_json::from_str::<Number>(text).map(Value::Number),
        _ => Err(serde_json::Error::custom("invalid JSON token")),
    }
}
