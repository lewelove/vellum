use serde_json::Value;

pub fn calculate_file_tag_subset_match(
    enriched: &mut Value,
    harvest: &[Value],
    subset_keys: &[String],
) {
    let mut matches = Vec::new();
    let Some(album_obj) = enriched.get("album").and_then(Value::as_object) else { return; };
    let Some(tracks_arr) = enriched.get("tracks").and_then(Value::as_array) else { return; };

    if tracks_arr.len() != harvest.len() {
        matches = vec![false; tracks_arr.len()];
    } else {
        for (idx, compiled_track) in tracks_arr.iter().enumerate() {
            let mut is_match = true;
            let p_tags = harvest[idx].get("tags").and_then(Value::as_object);
            let t_obj = compiled_track.as_object().unwrap();

            for key in subset_keys {
                let k_lower = key.to_lowercase();
                let mut compiled_val = t_obj.get(&k_lower);
                if compiled_val.is_none() { compiled_val = t_obj.get("keys").and_then(|keys| keys.get(&k_lower)); }
                if compiled_val.is_none() { compiled_val = album_obj.get(&k_lower); }
                if compiled_val.is_none() { compiled_val = album_obj.get("keys").and_then(|keys| keys.get(&k_lower)); }

                if let Some(v) = compiled_val {
                    let p_val = p_tags.and_then(|t| t.get(&k_lower)).and_then(Value::as_str).unwrap_or("");
                    if !compare_values(&k_lower, v, p_val) {
                        is_match = false;
                        break;
                    }
                }
            }
            matches.push(is_match);
        }
    }

    if let Some(tracks_arr_mut) = enriched.get_mut("tracks").and_then(Value::as_array_mut) {
        for (idx, compiled_track) in tracks_arr_mut.iter_mut().enumerate() {
            if let Some(info) = compiled_track.get_mut("info").and_then(Value::as_object_mut) {
                let match_val = matches.get(idx).copied().unwrap_or(false);
                info.insert("embedded_keys_subset_match".to_string(), Value::Bool(match_val));
            }
        }
    }
}

pub fn compare_values(key: &str, compiled: &Value, physical: &str) -> bool {
    let s_comp = match compiled {
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(|v| v.as_str().unwrap_or("").trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("; "),
        Value::Null => return physical.is_empty(),
        _ => compiled.to_string().replace('"', ""),
    };
    let (s_c, s_p) = (s_comp.trim(), physical.trim());
    let k_lower = key.to_lowercase();
    if k_lower == "tracknumber" || k_lower == "discnumber" {
        let parse = |s: &str| {
            s.split('/')
                .next()
                .unwrap_or("0")
                .trim()
                .parse::<u64>()
                .unwrap_or(0)
        };
        return parse(s_c) == parse(s_p);
    }
    s_c == s_p
}
