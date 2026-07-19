use crate::error::VellumError;
use serde_json::{Value, json};
use std::collections::{HashSet};
use std::path::Path;
use crate::types::toml_to_json;

pub fn load_manifests(
    album_root: &Path,
    manifest_names: Option<&Vec<Value>>,
) -> Result<serde_json::Map<String, Value>, VellumError> {
    let metadata_path = album_root.join("metadata.toml");
    if !metadata_path.exists() {
        return Err(VellumError::MissingPrimaryManifest { path: album_root.to_path_buf() });
    }

    let mut result_manifests = serde_json::Map::new();

    let primary_json = parse_single_manifest(&metadata_path, album_root, "metadata", None)?;
    let expected_tracks = primary_json.get("tracks").and_then(|t| t.as_array()).map_or(0, std::vec::Vec::len);
    result_manifests.insert("metadata".to_string(), primary_json);

    if let Some(names) = manifest_names {
        for m_val in names {
            if let Some(m_name) = m_val.as_str() {
                if m_name == "metadata.toml" { continue; }
                let m_path = album_root.join(m_name);
                if !m_path.exists() { continue; }
                
                let m_key = m_name.strip_suffix(".toml").unwrap_or(m_name);
                let aux_json = parse_single_manifest(&m_path, album_root, m_key, Some(expected_tracks))?;
                result_manifests.insert(m_key.to_string(), aux_json);
            }
        }
    }

    let local_path = album_root.join("local.toml");
    if local_path.exists() && !result_manifests.contains_key("local") {
        let local_json = parse_single_manifest(&local_path, album_root, "local", Some(expected_tracks))?;
        result_manifests.insert("local".to_string(), local_json);
    }

    Ok(result_manifests)
}

fn parse_single_manifest(
    path: &Path,
    album_root: &Path,
    name: &str,
    expected_tracks: Option<usize>,
) -> Result<Value, VellumError> {
    let content = std::fs::read_to_string(path)?;
    let parsed_toml = toml::from_str::<toml::Value>(&content)
        .map_err(|source| VellumError::ManifestParseError { path: path.to_path_buf(), source })?;
    
    let mut json_val = toml_to_json(parsed_toml);

    let album_obj = json_val.get("album").cloned().unwrap_or_else(|| json!({}));
    let album_obj = if name == "local" {
        json_val.get("local").cloned().unwrap_or(album_obj)
    } else {
        album_obj
    };

    let tracks_obj = if let Some(tracks_arr) = json_val.get_mut("tracks").and_then(Value::as_array_mut) {
        if tracks_arr.is_empty() && expected_tracks.is_some() {
            Value::Array(vec![])
        } else {
            if let Some(expected) = expected_tracks
                && tracks_arr.len() != expected
            {
                return Err(VellumError::TrackCountMismatch {
                    manifest: name.to_string(),
                    path: album_root.to_path_buf(),
                    primary_count: expected,
                    aux_count: tracks_arr.len(),
                });
            }

            let mut tuples = Vec::new();
            let mut seen_ids = HashSet::new();

            for (idx, t) in tracks_arr.iter_mut().enumerate() {
                let track_no = extract_strict_u32(t.get("tracknumber"), "tracknumber", None)
                    .map_err(|_| VellumError::MissingTrackIdentity {
                        manifest: name.to_string(),
                        path: album_root.to_path_buf(),
                        index: idx + 1,
                    })?;
                let disc_no = extract_strict_u32(t.get("discnumber"), "discnumber", Some(1))?;

                if !seen_ids.insert((disc_no, track_no)) {
                    return Err(VellumError::DuplicateTrackIdentity {
                        manifest: name.to_string(),
                        path: album_root.to_path_buf(),
                        disc: disc_no,
                        track: track_no,
                    });
                }
                tuples.push((disc_no, track_no, t.clone()));
            }

            tuples.sort_by_key(|(d, t, _)| (*d, *t));

            let sorted_tracks: Vec<Value> = tuples.into_iter().map(|(_, _, val)| val).collect();
            Value::Array(sorted_tracks)
        }
    } else {
        Value::Array(vec![])
    };

    Ok(json!({
        "album": album_obj,
        "tracks": tracks_obj
    }))
}

pub fn extract_strict_u32(val: Option<&Value>, name: &str, default: Option<u32>) -> Result<u32, VellumError> {
    let Some(v) = val else {
        return default.map_or_else(
            || Err(VellumError::InvalidIdentityFormat {
                field: name.to_string(),
                message: "Missing expected integer".to_string(),
            }),
            Ok,
        );
    };
    match v {
        Value::Number(n) => n
            .as_u64()
            .and_then(|i| u32::try_from(i).ok())
            .ok_or_else(|| VellumError::InvalidIdentityFormat {
                field: name.to_string(),
                message: "Value exceeds 32-bit integer limits".to_string(),
            }),
        Value::String(s) => {
            let base = s.split('/').next().unwrap_or("").trim();
            base.parse::<u32>().map_err(|_| VellumError::InvalidIdentityFormat {
                field: name.to_string(),
                message: format!("Cannot interpret string '{s}' as integer"),
            })
        }
        Value::Null => {
            default.map_or_else(
                || Err(VellumError::InvalidIdentityFormat {
                    field: name.to_string(),
                    message: "Field cannot be null".to_string(),
                }),
                Ok,
            )
        }
        _ => Err(VellumError::InvalidIdentityFormat {
            field: name.to_string(),
            message: "Unsupported data type found".to_string(),
        }),
    }
}
