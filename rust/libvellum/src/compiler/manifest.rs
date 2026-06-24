use crate::error::VellumError;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;
use crate::types::toml_to_json;

pub struct ManifestData {
    pub json: Value,
    pub manifests: Vec<String>,
}

pub fn load_and_merge(
    album_root: &Path,
    manifest_names: Option<&Vec<Value>>,
) -> Result<ManifestData, VellumError> {
    let metadata_path = album_root.join("metadata.toml");
    let _ = std::fs::metadata(&metadata_path)
        .map_err(|_| VellumError::MissingPrimaryManifest { path: album_root.to_path_buf() })?;

    let mut manifests = vec!["metadata.toml".to_string()];
    let content = std::fs::read_to_string(&metadata_path)?;

    let parsed_toml = toml::from_str::<toml::Value>(&content)
        .map_err(|source| VellumError::ManifestParseError { path: metadata_path.clone(), source })?;
    let mut metadata_json = toml_to_json(parsed_toml);

    if let Some(names) = manifest_names {
        for m_val in names {
            if let Some(m_name) = m_val.as_str() {
                let is_merged = merge_auxiliary_manifest(album_root, m_name, &mut metadata_json)?;
                if is_merged {
                    manifests.push(m_name.to_string());
                }
            }
        }
    }

    Ok(ManifestData {
        json: metadata_json,
        manifests,
    })
}

fn merge_auxiliary_manifest(
    album_root: &Path,
    m_name: &str,
    primary_json: &mut Value,
) -> Result<bool, VellumError> {
    let m_path = album_root.join(m_name);
    if !m_path.exists() {
        return Ok(false);
    }
    let m_content = std::fs::read_to_string(&m_path)?;
    let parsed_aux = toml::from_str::<toml::Value>(&m_content)
        .map_err(|source| VellumError::ManifestParseError { path: m_path.clone(), source })?;
    let mut m_json = toml_to_json(parsed_aux);
    
    if let Some(aux_album) = m_json.get_mut("album").and_then(Value::as_object_mut)
        && let Some(primary_album) = primary_json.get_mut("album").and_then(Value::as_object_mut) {
            for (k, v) in aux_album {
                if !primary_album.contains_key(k) {
                    primary_album.insert(k.clone(), v.clone());
                }
            }
        }

    if let Some(aux_tracks) = m_json.get_mut("tracks").and_then(Value::as_array_mut)
        .filter(|t| !t.is_empty())
    {
        merge_aux_tracks(album_root, m_name, aux_tracks, primary_json)?;
    }
    Ok(true)
}

fn merge_aux_tracks(
    album_root: &Path,
    m_name: &str,
    aux_tracks: &mut [Value],
    primary_json: &mut Value,
) -> Result<(), VellumError> {
    let primary_tracks = primary_json.get_mut("tracks")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| VellumError::MissingTracksBlock { path: album_root.to_path_buf() })?;

    if aux_tracks.len() != primary_tracks.len() {
        return Err(VellumError::TrackCountMismatch {
            manifest: m_name.to_string(),
            path: album_root.to_path_buf(),
            primary_count: primary_tracks.len(),
            aux_count: aux_tracks.len(),
        });
    }

    let mut seen_identities = HashSet::new();
    for (idx, aux_t) in aux_tracks.iter_mut().enumerate() {
        let a_obj = aux_t.as_object_mut().ok_or_else(|| VellumError::InvalidManifestEntry { 
            manifest: m_name.to_string(), 
            path: album_root.to_path_buf(), 
            index: idx + 1 
        })?;
        
        let track_no = extract_strict_u32(a_obj.get("tracknumber"), "tracknumber", None)
            .map_err(|_| VellumError::MissingTrackIdentity {
                manifest: m_name.to_string(),
                path: album_root.to_path_buf(),
                index: idx + 1,
            })?;
        
        let disc_no = extract_strict_u32(a_obj.get("discnumber"), "discnumber", Some(1))?;

        if !seen_identities.insert((disc_no, track_no)) {
            return Err(VellumError::DuplicateTrackIdentity {
                manifest: m_name.to_string(),
                path: album_root.to_path_buf(),
                disc: disc_no,
                track: track_no,
            });
        }

        let mut found = false;
        for prim_t in primary_tracks.iter_mut() {
            let p_track_no: u32 = extract_strict_u32(prim_t.get("tracknumber"), "tracknumber", None)?;
            let p_disc_no: u32 = extract_strict_u32(prim_t.get("discnumber"), "discnumber", Some(1))?;

            if track_no == p_track_no && disc_no == p_disc_no {
                if let Some(p_obj) = prim_t.as_object_mut() {
                    for (k, v) in a_obj {
                        if k != "tracknumber" && k != "discnumber"
                            && !p_obj.contains_key(k) {
                                p_obj.insert(k.clone(), v.clone());
                            }
                    }
                }
                found = true;
                break;
            }
        }
        if !found {
            return Err(VellumError::OrphanedAuxiliaryData {
                manifest: m_name.to_string(),
                path: album_root.to_path_buf(),
                disc: disc_no,
                track: track_no,
            });
        }
    }
    Ok(())
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
