pub mod assets;

use libvellum::error::VellumError;
use libvellum::compiler::manifest::{load_manifests, extract_strict_u32};
use libvellum::compiler::validation::validate_track_indices;
use crate::expand_path;
use crate::harvest;
use serde_json::{Value, json};
use std::path::Path;
use std::collections::HashMap;
use libvellum::models::CoverMetrics;
use serde::de::Error as _;
use base64::{Engine as _, engine::general_purpose::STANDARD};

struct PreparedContext {
    audio_files: Vec<std::path::PathBuf>,
    library_root: std::path::PathBuf,
}

fn is_virtual_album(album_root: &Path) -> bool {
    let local_path = album_root.join("local.toml");
    if let Ok(content) = std::fs::read_to_string(&local_path)
        && let Ok(parsed) = toml::from_str::<toml::Value>(&content)
        && let Some(local) = parsed.get("local")
        && let Some(virt) = local.get("virtual").and_then(toml::Value::as_bool)
    {
        return virt;
    }
    false
}

fn resolve_cover_data(
    album_root: &Path,
    config: &libvellum::lua::ResolvedConfig,
) -> (Value, Option<CoverMetrics>) {
    let main_cover_path = assets::resolve_cover_info(album_root);
    
    let mut cover_hash_address = String::new();
    let mut cover_file_info = Value::Null;

    if let Some(cp) = &main_cover_path {
        let content = std::fs::read(cp).unwrap_or_default();
        if !content.is_empty() {
            cover_hash_address = libvellum::utils::calculate_blake3_address(&content);
            let raw = blake3::hash(&content);
            let cover_hash_full = format!("blake3-{}", STANDARD.encode(raw.as_bytes()));
            let rel_path = libvellum::resolvers::rel_path(cp, album_root);
            if let Ok(info) = libvellum::utils::get_file_info(cp, &rel_path, false) {
                let mut info_map = info.as_object().unwrap().clone();
                info_map.insert("address".to_string(), json!(cover_hash_address));
                info_map.insert("hash".to_string(), json!(cover_hash_full));
                cover_file_info = Value::Object(info_map);
            }
        }
    }

    let loaded_image = assets::pregenerate_covers(config, main_cover_path.as_deref(), &cover_hash_address);
    let cover_metrics = resolve_cover_metrics(config, &cover_hash_address, loaded_image.as_ref());

    (cover_file_info, cover_metrics)
}

fn build_ctx_tracks(
    is_virtual: bool,
    primary_tracks: &[Value],
    audio_files: &[std::path::PathBuf],
    album_root: &Path,
) -> Result<(Vec<Value>, u64, Vec<Value>), VellumError> {
    let mut ctx_tracks = Vec::new();
    let mut duration_sum_ms = 0;
    let mut harvested_cache = Vec::new();

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
            harvested_cache.push(serde_json::to_value(&harvest)?);
        }
    }
    
    Ok((ctx_tracks, duration_sum_ms, harvested_cache))
}

fn build_final_tracks(
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

        let t_keys = track_keys_array.and_then(|arr| arr.get(i)).cloned().unwrap_or_else(|| json!({}));

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

fn parse_mandatory_album_fields(
    primary_album: &Value,
    album_root: &Path,
) -> Result<(String, String, String), VellumError> {
    let get_album_str = |k: &str| -> Result<String, VellumError> {
        let v = primary_album.get(k);
        if let Some(s) = v.and_then(Value::as_str) {
            if !s.is_empty() { return Ok(s.to_string()); }
        } else if let Some(n) = v.and_then(Value::as_number) {
            return Ok(n.to_string());
        }
        Err(VellumError::TypeMismatch {
            path: album_root.to_path_buf(),
            key: k.to_string(),
            expected_type: "string".to_string(),
            found_val: "missing or empty".to_string(),
        })
    };

    let albumartist = get_album_str("albumartist").or_else(|_| get_album_str("artist"))?;
    let album = get_album_str("album")?;
    let date = get_album_str("date")?;
    Ok((albumartist, album, date))
}

pub fn build(
    album_root: &Path,
    config: &libvellum::lua::ResolvedConfig,
) -> Result<Value, VellumError> {
    let manifest_names = config.app.compiler.manifests.as_ref().map(|v| v.iter().map(|s| Value::String(s.clone())).collect::<Vec<_>>());
    let parsed_manifests = load_manifests(album_root, manifest_names.as_ref())?;

    let primary_manifest = parsed_manifests.get("metadata").ok_or_else(|| VellumError::MissingPrimaryManifest { path: album_root.to_path_buf() })?;
    let primary_tracks = primary_manifest.get("tracks").and_then(Value::as_array).ok_or_else(|| VellumError::MissingTracksBlock { path: album_root.to_path_buf() })?;
    
    let (cover_file_info, cover_metrics) = resolve_cover_data(album_root, config);

    let PreparedContext { audio_files, library_root } = prepare_build_context(config, album_root);

    let is_virtual = is_virtual_album(album_root);

    if !is_virtual && audio_files.len() != primary_tracks.len() {
        return Err(VellumError::PhysicalInventoryMismatch {
            path: album_root.to_path_buf(),
            files_count: audio_files.len(),
            tracks_count: primary_tracks.len(),
        });
    }

    validate_track_indices(primary_tracks, album_root)?;

    let mut lock_manifests = HashMap::new();
    for (name, _) in &parsed_manifests {
        let file_name = if name == "local" { "local.toml".to_string() } else { format!("{name}.toml") };
        let abs_p = album_root.join(&file_name);
        if let Ok(info) = libvellum::utils::get_file_info(&abs_p, &file_name, false) {
            lock_manifests.insert(name.clone(), json!({ "file": info }));
        }
    }

    let total_discs = libvellum::resolvers::calculate_total_discs(primary_tracks);
    let total_tracks = primary_tracks.len() as u32;

    let (ctx_tracks, duration_sum_ms, harvested_cache) = build_ctx_tracks(is_virtual, primary_tracks, &audio_files, album_root)?;

    let config_json = serde_json::to_value(&config.app).unwrap_or_else(|_| json!({}));
    let id_str = libvellum::resolvers::rel_path(album_root, &library_root);

    let ctx = json!({
        "config": config_json,
        "id": id_str,
        "total_discs": total_discs,
        "total_tracks": total_tracks,
        "duration_milliseconds": duration_sum_ms,
        "cover_metrics": cover_metrics,
        "tracks": ctx_tracks,
    });

    let manifests_json = Value::Object(parsed_manifests.clone());

    let lua_res = libvellum::lua::get_or_init_lua_vm(&config.path, |engine| {
        engine.execute_dispatcher(&ctx, &manifests_json)
    }).map_err(|e| VellumError::ManifestParseError { path: album_root.to_path_buf(), source: toml::de::Error::custom(e.to_string()) })?; 

    let album_keys = lua_res.get("album").cloned().unwrap_or_else(|| json!({}));
    let track_keys_array = lua_res.get("tracks").and_then(Value::as_array);

    let empty_album = json!({});
    let primary_album = primary_manifest.get("album").unwrap_or(&empty_album);
    
    let (albumartist, album, date) = parse_mandatory_album_fields(primary_album, album_root)?;

    let date_added = if is_virtual {
        chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
    } else {
        libvellum::resolvers::resolve_album_info_date_added(album_root, primary_album, config)?
    };

    let info_obj = json!({
        "total_discs": total_discs,
        "total_tracks": total_tracks,
        "date_added": date_added,
        "duration_milliseconds": duration_sum_ms,
        "duration_formatted": libvellum::resolvers::format_ms(duration_sum_ms),
    });

    let mut album_obj = serde_json::Map::new();
    album_obj.insert("albumartist".to_string(), json!(albumartist));
    album_obj.insert("album".to_string(), json!(album));
    album_obj.insert("date".to_string(), json!(date));
    album_obj.insert("id".to_string(), json!(id_str));
    album_obj.insert("keys".to_string(), album_keys);
    album_obj.insert("info".to_string(), info_obj);
    album_obj.insert("manifests".to_string(), json!(lock_manifests));
    
    let covers_entry = if cover_file_info.is_null() {
        Value::Null
    } else {
        json!({ "main": { "file": cover_file_info } })
    };
    album_obj.insert("covers".to_string(), covers_entry);

    let final_tracks = build_final_tracks(
        primary_tracks,
        &albumartist,
        track_keys_array,
        &ctx_tracks,
        album_root,
        total_discs,
    )?;

    let mut final_json = serde_json::Map::new();
    final_json.insert("album".to_string(), Value::Object(album_obj));
    final_json.insert("tracks".to_string(), Value::Array(final_tracks));
    final_json.insert("ctx".to_string(), json!({
        "harvest": harvested_cache,
        "paths": {
            "album_root": album_root.to_string_lossy(),
            "project_root": config.path.parent().unwrap_or_else(|| Path::new(".")).to_string_lossy(),
            "library_root": library_root.to_string_lossy()
        }
    }));

    Ok(Value::Object(final_json))
}

fn prepare_build_context(
    config: &libvellum::lua::ResolvedConfig,
    album_root: &Path,
) -> PreparedContext {
    let exts: Vec<String> = config.app.manifest.audio_files.clone().unwrap_or_else(|| vec![".flac".to_string()]);
    let ext_refs: Vec<&str> = exts.iter().map(AsRef::as_ref).collect();
    let audio_files = libvellum::scanner::scan_audio_files(album_root, &ext_refs);

    let lib_root_raw = &config.app.storage.library;
    let library_root = expand_path(lib_root_raw)
        .canonicalize()
        .unwrap_or_else(|_| expand_path(lib_root_raw));

    PreparedContext { audio_files, library_root }
}

fn resolve_cover_metrics(
    config: &libvellum::lua::ResolvedConfig,
    c_hash: &str,
    loaded_image: Option<&image::DynamicImage>,
) -> Option<CoverMetrics> {
    if c_hash.is_empty() {
        return None;
    }
    
    let cache_str = &config.app.storage.cache;
    let cache_root = crate::expand_path(cache_str);
    let metrics_dir = cache_root.join("cover_data");
    std::fs::create_dir_all(&metrics_dir).ok();
    
    let metrics_path = metrics_dir.join(format!("{c_hash}.json"));
    
    let mut metrics = if metrics_path.exists() {
        std::fs::read_to_string(&metrics_path).map_or(None, |content| serde_json::from_str::<CoverMetrics>(&content).ok())
    } else { 
        None 
    }.unwrap_or_else(|| CoverMetrics {
        hash: c_hash.to_string(),
        entropy: None,
        chroma: None,
    });
    
    let mut needs_save = false;
    
    if let Some(img) = loaded_image {
        if metrics.chroma.is_none() {
            metrics.chroma = Some(libvellum::images::cover_chroma::calculate_chroma(img));
            needs_save = true;
        }
        if metrics.entropy.is_none() {
            metrics.entropy = Some(libvellum::images::cover_entropy::calculate_entropy(img));
            needs_save = true;
        }
    }
    
    if needs_save
        && let Ok(content) = serde_json::to_string(&metrics) {
            let _ = std::fs::write(&metrics_path, content);
        }
    
    Some(metrics)
}
