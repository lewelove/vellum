use crate::harvest::sanitize_key;
use indexmap::IndexMap;
use serde_json::Value;
use std::collections::HashSet;

fn format_toml_value(val: &Value) -> String {
    match val {
        Value::String(s) => serde_json::to_string(s).unwrap_or_default(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Array(arr) => {
            let formatted: Vec<String> = arr.iter().map(format_toml_value).collect();
            format!("[{}]", formatted.join(", "))
        }
        _ => serde_json::to_string(val).unwrap_or_default(),
    }
}

fn format_toml_value_with_cast(val: &Value, key: &str) -> String {
    if (key == "tracknumber" || key == "discnumber")
        && let Some(s) = val.as_str() {
            let clean = s.split('/').next().unwrap_or("").trim();
            if clean.parse::<u32>().is_ok() {
                return clean.to_string();
            }
        }
    format_toml_value(val)
}

pub fn render_toml_block(
    pool: &serde_json::Map<String, Value>,
    layout: Option<&IndexMap<String, toml::Value>>,
    level: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut layout_keys = HashSet::new();

    if let Some(lay) = layout {
        for key in lay.keys() {
            layout_keys.insert(sanitize_key(key));
        }

        for (key, meta) in lay {
            if let Some(meta_table) = meta.as_table() {
                let key_level = meta_table.get("level").and_then(|v| v.as_str()).unwrap_or("");
                if key_level == level {
                    let s_key = sanitize_key(key);
                    let val = pool.get(&s_key).or_else(|| pool.get(key));
                    
                    let rendered_val = val.map_or_else(
                        || "\"\"".to_string(), 
                        |v| format_toml_value_with_cast(v, &s_key)
                    );

                    let newline = meta_table
                        .get("newline")
                        .and_then(toml::Value::as_bool)
                        .unwrap_or(false);

                    if newline {
                        lines.push(String::new());
                    }

                    lines.push(format!("{s_key} = {rendered_val}"));
                }
            }
        }
    }

    let mut appendix_keys: Vec<String> = pool
        .keys()
        .filter(|k| {
            let s_k = sanitize_key(k);
            !layout_keys.contains(&s_k)
        })
        .cloned()
        .collect();
    appendix_keys.sort();

    for k in appendix_keys {
        if let Some(v) = pool.get(&k) {
            let s_k = sanitize_key(&k);
            lines.push(format!("{} = {}", s_k, format_toml_value_with_cast(v, &s_k)));
        }
    }

    lines
}
