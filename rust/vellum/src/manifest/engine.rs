use crate::harvest::sanitize_key;
use indexmap::IndexMap;
use libvellum::lua::config::ManifestKeyConfig;
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
    layout: Option<&IndexMap<String, ManifestKeyConfig>>,
    level: &str,
    target_manifest: &str,
) -> Vec<String> {
    let mut lines = Vec::new();
    let mut layout_keys = HashSet::new();

    if let Some(lay) = layout {
        for key in lay.keys() {
            layout_keys.insert(sanitize_key(key));
        }

        for (key, cfg) in lay {
            let manifests_str = cfg.manifests.as_deref().unwrap_or("metadata");
            let manifests: Vec<&str> = manifests_str.split(',').map(str::trim).filter(|s| !s.is_empty()).collect();
            let effective_manifests = if manifests.is_empty() { vec!["metadata"] } else { manifests };
            
            if !effective_manifests.contains(&target_manifest) {
                continue;
            }

            let key_level = &cfg.level;
            if key_level == level || key_level == &format!("{level}s") {
                let s_key = sanitize_key(key);
                let is_array = cfg.type_.as_deref() == Some("list");
                
                let rendered_val = pool.get(&s_key).or_else(|| pool.get(key)).map_or_else(
                    || if is_array { "[]".to_string() } else { "\"\"".to_string() },
                    |val| format_toml_value_with_cast(val, &s_key)
                );

                if cfg.newline {
                    lines.push(String::new());
                }

                lines.push(format!("{s_key} = {rendered_val}"));
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
