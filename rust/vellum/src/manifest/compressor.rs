use crate::harvest::sanitize_key;
use indexmap::IndexMap;
use libvellum::config::ManifestKeyConfig;
use serde_json::Value;
use std::collections::HashSet;

pub fn get_key_manifests(key: &str, layout: Option<&IndexMap<String, ManifestKeyConfig>>) -> Vec<String> {
    let s_key = sanitize_key(key);
    if let Some(lay) = layout {
        for (k, cfg) in lay {
            if sanitize_key(k) == s_key {
                if let Some(manifests_str) = &cfg.manifests {
                    return manifests_str.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                }
                break;
            }
        }
    }
    vec!["metadata".to_string()]
}

pub fn compress(
    mut raw_tracks: Vec<serde_json::Map<String, Value>>,
    manifest_layout: Option<&IndexMap<String, ManifestKeyConfig>>,
    target_manifest: &str,
) -> (
    serde_json::Map<String, Value>,
    Vec<serde_json::Map<String, Value>>,
) {
    if raw_tracks.is_empty() {
        return (serde_json::Map::new(), Vec::new());
    }

    let mut forced_track_keys = HashSet::new();
    if let Some(layout) = manifest_layout {
        for (key, cfg) in layout {
            if cfg.level == "track" || cfg.level == "tracks" {
                forced_track_keys.insert(sanitize_key(key));
            }
        }
    }

    for track in &mut raw_tracks {
        track.retain(|k, _| {
            let manifest_names = get_key_manifests(k, manifest_layout);
            manifest_names.contains(&target_manifest.to_string())
        });
    }

    let first_track = &raw_tracks[0];
    let mut candidate_keys: HashSet<String> = first_track.keys().cloned().collect();

    for track in raw_tracks.iter().skip(1) {
        candidate_keys.retain(|k| track.contains_key(k));
    }

    let mut album_pool = serde_json::Map::new();
    let mut keys_to_promote = Vec::new();

    for key in candidate_keys {
        let is_identical = raw_tracks
            .iter()
            .all(|t| t.get(&key) == first_track.get(&key));

        if is_identical {
            let s_key = sanitize_key(&key);
            if forced_track_keys.contains(&s_key) {
                continue;
            }
            keys_to_promote.push(key.clone());
            album_pool.insert(key.clone(), first_track.get(&key).unwrap().clone());
        }
    }

    for track in &mut raw_tracks {
        for k in &keys_to_promote {
            track.remove(k);
        }
    }

    (album_pool, raw_tracks)
}
