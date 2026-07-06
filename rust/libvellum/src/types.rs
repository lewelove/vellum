use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;
use crate::error::VellumError;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VellumDataType {
    Boolean,
    Integer,
    Float,
    Array,
    Datetime,
    Path,
    Url,
    Object,
    #[serde(other)]
    String,
}

#[must_use]
pub fn get_raw_value<'a>(source: &'a Value, key: &str, args: &str) -> Option<&'a Value> {
    let check = |k: &str| -> Option<&'a Value> {
        let v = source.get(k)?;
        match v {
            Value::Null => None,
            Value::String(s) if s.trim().is_empty() => None,
            Value::Array(a) if a.is_empty() => None,
            _ => Some(v),
        }
    };

    if let Some(v) = check(key) {
        return Some(v);
    }
    
    if !args.is_empty() {
        for fallback in args.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if let Some(v) = check(fallback) {
                return Some(v);
            }
        }
    }
    None
}

#[must_use]
pub fn parse_toml_datetime(s: &str) -> bool {
    let s = s.trim();
    if chrono::DateTime::parse_from_rfc3339(s).is_ok() {
        return true;
    }
    if chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S").is_ok() {
        return true;
    }
    if chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S").is_ok() {
        return true;
    }
    if chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").is_ok() {
        return true;
    }
    if chrono::NaiveDate::parse_from_str(&format!("{s}-01"), "%Y-%m-%d").is_ok() {
        return true;
    }
    if chrono::NaiveDate::parse_from_str(&format!("{s}-01-01"), "%Y-%m-%d").is_ok() {
        return true;
    }
    false
}

#[must_use]
pub fn parse_time(val: Option<&Value>) -> String {
    let now = chrono::Utc::now();
    let dt = match val {
        Some(Value::Number(n)) => n.as_i64().map_or(now, |i| {
            if i > 253_402_300_799 {
                chrono::DateTime::from_timestamp_millis(i).unwrap_or(now)
            } else {
                chrono::DateTime::from_timestamp(i, 0).unwrap_or(now)
            }
        }),
        Some(Value::String(s)) => {
            let trimmed = s.trim();
            trimmed.parse::<i64>().map_or_else(
                |_| {
                    chrono::DateTime::parse_from_rfc3339(trimmed)
                        .map(Into::into)
                        .unwrap_or(now)
                },
                |i| {
                    if i > 253_402_300_799 {
                        chrono::DateTime::from_timestamp_millis(i).unwrap_or(now)
                    } else {
                        chrono::DateTime::from_timestamp(i, 0).unwrap_or(now)
                    }
                },
            )
        }
        _ => now,
    };
    dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

#[must_use]
pub fn toml_to_json(val: toml::Value) -> Value {
    match val {
        toml::Value::String(s) => Value::String(s),
        toml::Value::Integer(i) => Value::Number(i.into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(f)
            .map_or(Value::Null, Value::Number),
        toml::Value::Boolean(b) => Value::Bool(b),
        toml::Value::Datetime(dt) => Value::String(dt.to_string()),
        toml::Value::Array(arr) => Value::Array(arr.into_iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let mut map = serde_json::Map::new();
            for (k, v) in table {
                map.insert(k, toml_to_json(v));
            }
            Value::Object(map)
        }
    }
}

pub fn resolve_type_datetime(source: &Value, key: &str, args: &str, path: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    raw.map_or_else(
        || Ok(Value::Null),
        |v| match v {
            Value::String(s) => {
                if parse_toml_datetime(s) {
                    Ok(json!(s.trim()))
                } else {
                    Err(VellumError::TypeMismatch {
                        path: path.to_path_buf(),
                        key: key.to_string(),
                        expected_type: "datetime".to_string(),
                        found_val: s.clone(),
                    })
                }
            }
            _ => Err(VellumError::TypeMismatch {
                path: path.to_path_buf(),
                key: key.to_string(),
                expected_type: "datetime".to_string(),
                found_val: v.to_string(),
            }),
        }
    )
}

pub fn resolve_type_string(source: &Value, key: &str, args: &str, default: &str, path: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::String(s)) => Ok(json!(s.trim())),
        Some(v) => Err(VellumError::TypeMismatch {
            path: path.to_path_buf(),
            key: key.to_string(),
            expected_type: "string".to_string(),
            found_val: v.to_string(),
        }),
        None => {
            if default.is_empty() {
                Ok(Value::Null)
            } else {
                Ok(json!(default))
            }
        }
    }
}

pub fn resolve_type_array(source: &Value, key: &str, args: &str, path: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::Array(arr)) => {
            let mut list = Vec::new();
            for item in arr {
                match item {
                    Value::String(s) => list.push(s.trim().to_string()),
                    _ => return Err(VellumError::TypeMismatch {
                        path: path.to_path_buf(),
                        key: key.to_string(),
                        expected_type: "array of strings".to_string(),
                        found_val: item.to_string(),
                    }),
                }
            }
            Ok(json!(list))
        }
        Some(Value::String(s)) => {
            let list: Vec<String> = s.split(';').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
            Ok(json!(list))
        }
        Some(v) => Err(VellumError::TypeMismatch {
            path: path.to_path_buf(),
            key: key.to_string(),
            expected_type: "array".to_string(),
            found_val: v.to_string(),
        }),
        None => Ok(json!(Vec::<String>::new())),
    }
}

pub fn resolve_type_integer(source: &Value, key: &str, args: &str, path: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::Number(n)) => {
            n.as_i64().map_or_else(
                || Err(VellumError::TypeMismatch {
                    path: path.to_path_buf(),
                    key: key.to_string(),
                    expected_type: "integer".to_string(),
                    found_val: n.to_string(),
                }),
                |i| Ok(json!(i))
            )
        }
        Some(v) => Err(VellumError::TypeMismatch {
            path: path.to_path_buf(),
            key: key.to_string(),
            expected_type: "integer".to_string(),
            found_val: v.to_string(),
        }),
        None => Ok(Value::Null),
    }
}

pub fn resolve_type_float(source: &Value, key: &str, args: &str, path: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::Number(n)) => {
            n.as_f64().map_or_else(
                || Err(VellumError::TypeMismatch {
                    path: path.to_path_buf(),
                    key: key.to_string(),
                    expected_type: "float".to_string(),
                    found_val: n.to_string(),
                }),
                |f| Ok(json!(f))
            )
        }
        Some(v) => Err(VellumError::TypeMismatch {
            path: path.to_path_buf(),
            key: key.to_string(),
            expected_type: "float".to_string(),
            found_val: v.to_string(),
        }),
        None => Ok(Value::Null),
    }
}

pub fn resolve_type_boolean(source: &Value, key: &str, args: &str, path: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::Bool(b)) => Ok(json!(b)),
        Some(v) => Err(VellumError::TypeMismatch {
            path: path.to_path_buf(),
            key: key.to_string(),
            expected_type: "boolean".to_string(),
            found_val: v.to_string(),
        }),
        None => Ok(Value::Null),
    }
}

pub fn resolve_type_path(source: &Value, key: &str, args: &str, album_root: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::String(s)) => {
            Ok(json!(s.trim()))
        }
        Some(v) => Err(VellumError::TypeMismatch {
            path: album_root.to_path_buf(),
            key: key.to_string(),
            expected_type: "path (string)".to_string(),
            found_val: v.to_string(),
        }),
        None => Ok(Value::Null),
    }
}

pub fn resolve_type_url(source: &Value, key: &str, args: &str, path: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::String(s)) => {
            let url_str = s.trim();
            if url_str.starts_with("http://") || url_str.starts_with("https://") {
                Ok(json!(url_str))
            } else {
                Err(VellumError::TypeMismatch {
                    path: path.to_path_buf(),
                    key: key.to_string(),
                    expected_type: "url".to_string(),
                    found_val: url_str.to_string(),
                })
            }
        }
        Some(v) => Err(VellumError::TypeMismatch {
            path: path.to_path_buf(),
            key: key.to_string(),
            expected_type: "url".to_string(),
            found_val: v.to_string(),
        }),
        None => Ok(Value::Null),
    }
}

pub fn resolve_type_object(source: &Value, key: &str, args: &str, path: &Path) -> Result<Value, VellumError> {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::Object(obj)) => Ok(Value::Object(obj.clone())),
        Some(v) => Err(VellumError::TypeMismatch {
            path: path.to_path_buf(),
            key: key.to_string(),
            expected_type: "object (inline table)".to_string(),
            found_val: v.to_string(),
        }),
        None => Ok(Value::Null),
    }
}
