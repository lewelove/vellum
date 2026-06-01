use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::Path;

#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum VellumDataType {
    Boolean,
    Integer,
    Float,
    String,
    Array,
    Datetime,
    Path,
    Url,
    Object,
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
pub fn resolve_type_datetime(source: &Value, key: &str, args: &str) -> Value {
    let raw = get_raw_value(source, key, args);
    json!(parse_time(raw))
}

#[must_use]
pub fn resolve_type_string(source: &Value, key: &str, args: &str, default: &str) -> Value {
    get_raw_value(source, key, args).map_or_else(
        || json!(default),
        |v| match v {
            Value::String(s) => json!(s.trim()),
            Value::Array(arr) => json!(arr.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("; ")),
            _ => json!(v.to_string().replace('"', "").trim()),
        }
    )
}

#[must_use]
pub fn resolve_type_array(source: &Value, key: &str, args: &str) -> Value {
    get_raw_value(source, key, args).map_or_else(
        || json!(Vec::<String>::new()),
        |v| match v {
            Value::Array(arr) => {
                let list: Vec<String> = arr.iter().filter_map(|x| x.as_str().map(|s| s.trim().to_string())).filter(|s| !s.is_empty()).collect();
                json!(list)
            }
            Value::String(s) => {
                let list: Vec<String> = s.split(';').map(|x| x.trim().to_string()).filter(|x| !x.is_empty()).collect();
                json!(list)
            }
            _ => json!([v.to_string().replace('"', "").trim()]),
        }
    )
}

#[must_use]
pub fn resolve_type_integer(source: &Value, key: &str, args: &str) -> Value {
    get_raw_value(source, key, args).map_or_else(
        || json!(0),
        |v| match v {
            Value::Number(n) => json!(n.as_i64().unwrap_or(0)),
            Value::String(s) => json!(s.trim().parse::<i64>().unwrap_or(0)),
            _ => json!(0),
        }
    )
}

#[must_use]
pub fn resolve_type_float(source: &Value, key: &str, args: &str) -> Value {
    get_raw_value(source, key, args).map_or_else(
        || json!(0.0),
        |v| match v {
            Value::Number(n) => json!(n.as_f64().unwrap_or(0.0)),
            Value::String(s) => json!(s.trim().parse::<f64>().unwrap_or(0.0)),
            _ => json!(0.0),
        }
    )
}

#[must_use]
pub fn resolve_type_boolean(source: &Value, key: &str, args: &str) -> Value {
    get_raw_value(source, key, args).map_or_else(
        || json!(false),
        |v| match v {
            Value::Bool(b) => json!(*b),
            Value::String(s) => {
                let s = s.trim().to_lowercase();
                json!(s == "true" || s == "1" || s == "yes")
            }
            Value::Number(n) => json!(n.as_i64().unwrap_or(0) > 0),
            _ => json!(false),
        }
    )
}

#[must_use]
pub fn resolve_type_path(source: &Value, key: &str, args: &str, _album_root: &Path) -> Value {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::String(s)) => {
            json!(s.trim())
        }
        _ => Value::Null,
    }
}

#[must_use]
pub fn resolve_type_url(source: &Value, key: &str, args: &str) -> Value {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::String(s)) => {
            let url_str = s.trim();
            if url_str.starts_with("http://") || url_str.starts_with("https://") {
                json!(url_str)
            } else {
                Value::Null
            }
        }
        _ => Value::Null,
    }
}

#[must_use]
pub fn resolve_type_object(source: &Value, key: &str, args: &str) -> Value {
    let raw = get_raw_value(source, key, args);
    match raw {
        Some(Value::Object(obj)) => Value::Object(obj.clone()),
        _ => Value::Null,
    }
}
