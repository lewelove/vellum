use crate::compile::utils::sort_json_keys;
use libvellum::compiler::manifest::extract_strict_u32;
use libvellum::compiler::validation::validate_track_indices;
use libvellum::error::VellumError;
use crate::harvest;
use serde_json::{Value, json};
use std::path::Path;

pub fn build_ctx_tracks(
    is_virtual: bool,
    primary_tracks: &[Value],
    audio_files: &[std::path::PathBuf],
    album_root: &Path,
) -> Result<(Vec<Value>, u64), VellumError> {
    let mut ctx_tracks = Vec::new();
    let mut duration_sum_ms = 0;

    for (idx, _track_val) in primary_tracks.iter().enumerate() {
        if is_virtual {
            ctx_tracks.push(json!({
                "sample_rate": 0,
                "bits_per_sample": 0,
                "bitrate_kbps": 0,
                "encoding": "",
                "channels": 0,
                "duration_milliseconds": 0,
                "file": {
                    "path": "",
                    "mtime": 0,
                    "byte_size": 0
                },
                "embedded": {}
            }));
        } else {
            let file_path = &audio_files[idx];
            let harvest = harvest::harvest_file(file_path).map_err(|source| VellumError::HarvestError { path: file_path.clone(), source })?;
            let rel_path = libvellum::resolvers::rel_path(file_path, album_root);
            let file_info = libvellum::utils::get_file_info(file_path, &rel_path, false).unwrap_or_else(|_| json!({}));
            
            ctx_tracks.push(json!({
                "sample_rate": harvest.physics.sample_rate,
                "bits_per_sample": harvest.physics.bit_depth.unwrap_or(0),
                "bitrate_kbps": harvest.physics.audio_bitrate,
                "encoding": harvest.physics.format,
                "channels": harvest.physics.channels,
                "duration_milliseconds": harvest.physics.duration_ms,
                "file": file_info,
                "embedded": harvest.tags
            }));

            duration_sum_ms += harvest.physics.duration_ms;
        }
    }
    
    Ok((ctx_tracks, duration_sum_ms))
}

pub fn build_final_tracks(
    primary_tracks: &[Value],
    albumartist: &str,
    track_keys_array: Option<&Vec<Value>>,
    ctx_tracks: &[Value],
    album_root: &Path,
    total_discs: u32,
) -> Result<Vec<Value>, VellumError> {
    let mut final_tracks = Vec::new();

    for (i, t_val) in primary_tracks.iter().enumerate() {
        let discnumber = extract_strict_u32(t_val.get("discnumber"), "discnumber", Some(1))?;
        let tracknumber = extract_strict_u32(t_val.get("tracknumber"), "tracknumber", None)?;
        
        let t_artist = t_val.get("artist").and_then(Value::as_str).unwrap_or(albumartist).to_string();
        let t_title = t_val.get("title").and_then(Value::as_str).ok_or_else(|| VellumError::TypeMismatch {
            path: album_root.to_path_buf(),
            key: "title".to_string(),
            expected_type: "string".to_string(),
            found_val: "missing".to_string(),
        })?.to_string();

        let mut t_keys = track_keys_array.and_then(|arr| arr.get(i)).cloned().unwrap_or_else(|| json!({}));
        sort_json_keys(&mut t_keys);

        let ctx_track = &ctx_tracks[i];

        let mut t_obj = serde_json::Map::new();
        t_obj.insert("discnumber".to_string(), json!(discnumber));
        t_obj.insert("tracknumber".to_string(), json!(tracknumber));
        t_obj.insert("artist".to_string(), json!(t_artist));
        t_obj.insert("title".to_string(), json!(t_title));
        t_obj.insert("keys".to_string(), t_keys);
        
        let t_info = json!({
            "sample_rate": ctx_track.get("sample_rate").cloned().unwrap_or_else(|| json!(0)),
            "bits_per_sample": ctx_track.get("bits_per_sample").cloned().unwrap_or_else(|| json!(0)),
            "bitrate_kbps": ctx_track.get("bitrate_kbps").cloned().unwrap_or_else(|| json!(0)),
            "encoding": ctx_track.get("encoding").cloned().unwrap_or_else(|| json!("")),
            "channels": ctx_track.get("channels").cloned().unwrap_or_else(|| json!(0)),
            "duration_milliseconds": ctx_track.get("duration_milliseconds").cloned().unwrap_or_else(|| json!(0)),
            "duration_formatted": libvellum::resolvers::format_ms(ctx_track.get("duration_milliseconds").and_then(Value::as_u64).unwrap_or(0)),
        });
        t_obj.insert("info".to_string(), t_info);
        t_obj.insert("file".to_string(), ctx_track.get("file").cloned().unwrap_or_else(|| json!({})));

        let lyrics_path_str = t_val.get("lyrics_path").and_then(Value::as_str).map(ToString::to_string)
            .or_else(|| libvellum::resolvers::resolve_lyrics_path(album_root, tracknumber, discnumber, total_discs));
        
        if let Some(lp) = lyrics_path_str {
            let abs_lp = album_root.join(&lp);
            if let Ok(file_info) = libvellum::utils::get_file_info(&abs_lp, &lp, false) {
                t_obj.insert("lyrics".to_string(), json!({ "file": file_info }));
            }
        }

        final_tracks.push(Value::Object(t_obj));
    }
    
    Ok(final_tracks)
}

pub fn validate_audio_files(
    is_virtual: bool,
    audio_files: &[std::path::PathBuf],
    primary_tracks: &[Value],
    album_root: &Path,
) -> Result<(), VellumError> {
    if !is_virtual && audio_files.len() != primary_tracks.len() {
        return Err(VellumError::PhysicalInventoryMismatch {
            path: album_root.to_path_buf(),
            files_count: audio_files.len(),
            tracks_count: primary_tracks.len(),
        });
    }
    validate_track_indices(primary_tracks, album_root)?;
    Ok(())
}
