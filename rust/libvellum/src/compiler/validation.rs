use crate::error::VellumError;
use crate::compiler::manifest::extract_strict_u32;
use serde_json::{Map, Value};
use std::collections::HashSet;
use std::path::Path;

pub fn validate_track_indices(entries: &[Value], root: &Path) -> Result<(), VellumError> {
    let mut seen_ids = HashSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let t = extract_strict_u32(entry.get("tracknumber"), "tracknumber", None)
            .map_err(|_| VellumError::MissingTrackIdentity {
                manifest: "metadata.toml".to_string(),
                path: root.to_path_buf(),
                index: idx + 1,
            })?;
        let d = extract_strict_u32(entry.get("discnumber"), "discnumber", Some(1))?;

        if !seen_ids.insert((d, t)) {
            return Err(VellumError::DuplicateTrackIdentity {
                manifest: "metadata.toml".to_string(),
                path: root.to_path_buf(),
                disc: d,
                track: t,
            });
        }
    }
    Ok(())
}

pub fn validate_album_level_keys(
    album_source: &Value,
    track_entries: &[Value],
    registry: &Map<String, Value>,
    album_root: &Path,
) -> Result<(), VellumError> {
    for (key, meta) in registry {
        if meta.get("level").and_then(Value::as_str) != Some("album") {
            continue;
        }
        
        let mut seen_values: Vec<(Value, String)> = Vec::new();
        
        let check_val = |v: &Value| -> bool {
            match v {
                Value::Null => false,
                Value::String(s) => !s.trim().is_empty(),
                Value::Array(a) => !a.is_empty(),
                _ => true,
            }
        };

        if let Some(v) = album_source.get(key).filter(|v| check_val(v)) {
            seen_values.push((v.clone(), "album section".to_string()));
        }

        for (idx, track) in track_entries.iter().enumerate() {
            if let Some(v) = track.get(key).filter(|v| check_val(v)) {
                if let Some((first_val, source_name)) = seen_values.first() {
                    if v != first_val {
                        return Err(VellumError::GlobalKeyConflict {
                            path: Box::new(album_root.to_path_buf()),
                            key: Box::new(key.clone()),
                            val1: Box::new(first_val.to_string()),
                            source1: Box::new(source_name.clone()),
                            val2: Box::new(v.to_string()),
                            index: idx + 1,
                        });
                    }
                } else {
                    seen_values.push((v.clone(), format!("track {}", idx + 1)));
                }
            }
        }
    }
    Ok(())
}

pub fn merge_local_registry(album_root: &Path, registry: &mut Map<String, Value>) {
    let local_toml_path = album_root.join("local.toml");
    if local_toml_path.exists()
        && let Ok(local_content) = std::fs::read_to_string(&local_toml_path)
            && let Ok(local_toml) = toml::from_str::<toml::Value>(&local_content)
                && let Ok(local_json) = serde_json::to_value(local_toml)
                    && let Some(local_keys) = local_json
                        .get("compiler")
                        .and_then(|c| c.get("keys"))
                        .and_then(Value::as_object)
                    {
                        for (k, v) in local_keys {
                            if let Some(existing) = registry.get_mut(k) {
                                if let (Some(existing_obj), Some(new_obj)) = (existing.as_object_mut(), v.as_object()) {
                                    for (nk, nv) in new_obj {
                                        existing_obj.insert(nk.clone(), nv.clone());
                                    }
                                } else {
                                    registry.insert(k.clone(), v.clone());
                                }
                            } else {
                                registry.insert(k.clone(), v.clone());
                            }
                        }
                    }
}
