pub mod assets;
pub mod context;

use libvellum::error::VellumError;
use libvellum::compiler::manifest::{load_and_merge, extract_strict_u32};
use libvellum::compiler::validation::{validate_track_indices, validate_album_level_keys, merge_local_registry};
use crate::compile::builder::context::{AlbumContext, TrackContext};
use crate::compile::resolvers;
use crate::expand_path;
use crate::harvest;
use serde_json::{Value, json, Map};
use std::path::Path;
use libvellum::models::CoverMetrics;

struct PreparedContext {
    audio_files: Vec<std::path::PathBuf>,
    registry: Map<String, Value>,
    library_root: std::path::PathBuf,
}

pub fn build(
    album_root: &Path,
    project_root: &Path,
    config: &Value,
    manifest_cfg: &Value,
    _active_flags: &[String],
) -> Result<Value, VellumError> {
    let manifest_names = config.get("compiler").and_then(|c| c.get("manifests")).and_then(Value::as_array);
    let manifest_data = load_and_merge(album_root, manifest_names)?;

    let main_cover_path = assets::resolve_cover_info(album_root);
    
    let mut cover_hash_address = String::new();
    if let Some(cp) = &main_cover_path {
        let content = std::fs::read(cp).unwrap_or_default();
        if !content.is_empty() {
            cover_hash_address = libvellum::utils::calculate_blake3_address(&content);
        }
    }

    let loaded_image = assets::pregenerate_covers(config, main_cover_path.as_deref(), &cover_hash_address);
    let cover_metrics = resolve_cover_metrics(config, &cover_hash_address, loaded_image.as_ref());

    let PreparedContext { audio_files, registry, library_root } = prepare_build_context(config, manifest_cfg, album_root)?;

    let track_entries = manifest_data.json
        .get("tracks")
        .and_then(Value::as_array)
        .ok_or_else(|| VellumError::MissingTracksBlock { path: album_root.to_path_buf() })?;

    if audio_files.len() != track_entries.len() {
        return Err(VellumError::PhysicalInventoryMismatch {
            path: album_root.to_path_buf(),
            files_count: audio_files.len(),
            tracks_count: track_entries.len(),
        });
    }

    validate_track_indices(track_entries, album_root)?;

    let mut manifest_entries = Vec::new();
    for m in &manifest_data.manifests {
        let abs_p = album_root.join(m);
        if let Ok(info) = libvellum::utils::get_file_info(&abs_p, m, true) {
            manifest_entries.push(json!({ "file": info }));
        }
    }

    let mut covers_entry = serde_json::Map::new();
    if let Some(cp) = &main_cover_path {
        let rel_path = libvellum::resolvers::rel_path(cp, album_root);
        if let Ok(info) = libvellum::utils::get_file_info(cp, &rel_path, true) {
            covers_entry.insert("main".to_string(), json!({ "file": info }));
        }
    }

    let empty_obj = json!({});
    let album_source = manifest_data.json.get("album").unwrap_or(&empty_obj);

    validate_album_level_keys(album_source, track_entries, &registry, album_root)?;

    let (final_tracks, harvested_cache) = process_tracks(
        audio_files,
        track_entries,
        album_source,
        album_root,
        &registry,
    )?;

    let album_ctx = AlbumContext {
        source: album_source,
        tracks: &final_tracks,
        album_root,
        library_root: &library_root,
        cover_metrics: cover_metrics.as_ref(),
        config,
        manifests: manifest_entries,
        covers: Value::Object(covers_entry),
    };

    let album_obj = build_album(&album_ctx, &registry)?;

    let mut final_json = serde_json::Map::new();
    final_json.insert("album".to_string(), album_obj);
    final_json.insert("tracks".to_string(), Value::Array(final_tracks));
    final_json.insert("ctx".to_string(), json!({
        "config": config,
        "registry": registry,
        "metadata": manifest_data.json,
        "harvest": harvested_cache,
        "paths": {
            "album_root": album_root.to_string_lossy(),
            "project_root": project_root.to_string_lossy(),
            "library_root": library_root.to_string_lossy()
        }
    }));

    Ok(Value::Object(final_json))
}

fn prepare_build_context(
    config: &Value,
    manifest_cfg: &Value,
    album_root: &Path,
) -> Result<PreparedContext, VellumError> {
    let exts: Vec<&str> = manifest_cfg
        .get("supported_extensions")
        .and_then(Value::as_array)
        .map_or_else(
            || vec![".flac"],
            |arr| arr.iter().filter_map(Value::as_str).collect(),
        );

    let audio_files = libvellum::scanner::scan_audio_files(album_root, &exts);

    let lib_root_raw = config
        .get("storage")
        .and_then(|s| s.get("library_root"))
        .and_then(Value::as_str)
        .unwrap_or(".");
    let library_root = expand_path(lib_root_raw)
        .canonicalize()
        .unwrap_or_else(|_| expand_path(lib_root_raw));

    let keys_config = config
        .get("compiler")
        .and_then(|c| c.get("keys"))
        .and_then(Value::as_object)
        .ok_or_else(|| VellumError::MissingCompilerRegistry)?;

    let mut registry = Map::new();

    if let Some(albums) = keys_config.get("album").and_then(Value::as_object) {
        for (k, v) in albums {
            if let Some(mut obj) = v.as_object().cloned() {
                obj.insert("level".to_string(), json!("album"));
                registry.insert(k.clone(), Value::Object(obj));
            }
        }
    }

    if let Some(tracks) = keys_config.get("tracks").and_then(Value::as_object) {
        for (k, v) in tracks {
            if let Some(mut obj) = v.as_object().cloned() {
                obj.insert("level".to_string(), json!("tracks"));
                registry.insert(k.clone(), Value::Object(obj));
            }
        }
    }

    merge_local_registry(album_root, &mut registry);

    Ok(PreparedContext { audio_files, registry, library_root })
}

fn resolve_cover_metrics(
    config: &Value,
    c_hash: &str,
    loaded_image: Option<&image::DynamicImage>,
) -> Option<CoverMetrics> {
    if c_hash.is_empty() {
        return None;
    }
    
    let cache_str = config.get("storage").and_then(|s| s.get("cache")).and_then(Value::as_str).unwrap_or("~/.cache/vellum");
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

fn process_tracks(
    audio_files: Vec<std::path::PathBuf>,
    track_entries: &[Value],
    album_source: &Value,
    album_root: &Path,
    registry: &Map<String, Value>,
) -> Result<(Vec<Value>, Vec<Value>), VellumError> {
    let mut harvested_spine = Vec::new();
    for path in audio_files {
        harvested_spine.push(harvest::harvest_file(&path).map_err(|source| VellumError::HarvestError { path: path.clone(), source })?);
    }

    let mut total_discs = 1;
    for t in track_entries {
        if let Ok(d) = extract_strict_u32(t.get("discnumber"), "discnumber", Some(1)) {
            total_discs = total_discs.max(d);
        }
    }

    let mut final_tracks = Vec::new();
    let mut harvested_cache = Vec::new();

    for (idx, h_data) in harvested_spine.into_iter().enumerate() {
        let track_number: u32 = extract_strict_u32(track_entries[idx].get("tracknumber"), "tracknumber", None)
            .map_err(|_| VellumError::MissingTrackIdentity {
                manifest: "metadata.toml".to_string(),
                path: album_root.to_path_buf(),
                index: idx + 1,
            })?;
        let disc_number: u32 = extract_strict_u32(track_entries[idx].get("discnumber"), "discnumber", Some(1))?;

        let t_ctx = TrackContext {
            track_number,
            disc_number,
            harvest: &h_data,
            source: &track_entries[idx],
            album_source,
            album_root,
        };

        let t_obj = build_track(&t_ctx, total_discs, registry)?;
        final_tracks.push(t_obj);
        harvested_cache.push(serde_json::to_value(h_data)?);
    }

    Ok((final_tracks, harvested_cache))
}

fn build_track(
    ctx: &TrackContext,
    total_discs: u32,
    registry: &Map<String, Value>,
) -> Result<Value, VellumError> {
    let mut obj = serde_json::Map::new();

    obj.insert("discnumber".to_string(), json!(ctx.disc_number));
    obj.insert("tracknumber".to_string(), json!(ctx.track_number));
    obj.insert("artist".to_string(), resolvers::resolve_top_level_track_key("artist", ctx)?);
    obj.insert("title".to_string(), resolvers::resolve_top_level_track_key("title", ctx)?);

    let lyrics_path_str = ctx.source.get("lyrics_path").and_then(Value::as_str).map(ToString::to_string)
        .or_else(|| libvellum::resolvers::resolve_lyrics_path(ctx.album_root, ctx.track_number, ctx.disc_number, total_discs));
    
    if let Some(lp) = lyrics_path_str {
        let abs_lp = ctx.album_root.join(&lp);
        if let Ok(file_info) = libvellum::utils::get_file_info(&abs_lp, &lp, true) {
            let mut l_obj = serde_json::Map::new();
            l_obj.insert("file".to_string(), file_info);
            obj.insert("lyrics".to_string(), Value::Object(l_obj));
        }
    }

    let mut keys = serde_json::Map::new();
    for (key, meta) in registry {
        let level = meta.get("level").and_then(Value::as_str).unwrap_or("");
        if level != "tracks" && level != "track" { continue; }
        if ["title", "artist", "tracknumber", "discnumber"].contains(&key.as_str()) { continue; }
        let val = resolvers::resolve_track_key(key, meta, ctx)?.unwrap_or(Value::Null);
        keys.insert(key.clone(), val);
    }
    obj.insert("keys".to_string(), Value::Object(keys));

    let mut info = serde_json::Map::new();
    info.insert("sample_rate".to_string(), json!(ctx.harvest.physics.sample_rate));
    info.insert("bits_per_sample".to_string(), json!(ctx.harvest.physics.bit_depth.unwrap_or(0)));
    info.insert("bitrate_kbps".to_string(), json!(ctx.harvest.physics.bit_depth.unwrap_or(0)));
    info.insert("encoding".to_string(), json!(ctx.harvest.physics.format));
    info.insert("channels".to_string(), json!(ctx.harvest.physics.channels));
    info.insert("duration_milliseconds".to_string(), json!(ctx.harvest.physics.duration_ms));
    info.insert("duration_formatted".to_string(), json!(libvellum::resolvers::format_ms(ctx.harvest.physics.duration_ms)));
    info.insert("embedded_keys_subset_match".to_string(), json!(false));
    
    obj.insert("info".to_string(), Value::Object(info));

    let track_rel_path = libvellum::resolvers::rel_path(&ctx.harvest.path, ctx.album_root);
    if let Ok(mut file_info) = libvellum::utils::get_file_info(&ctx.harvest.path, &track_rel_path, false) {
        if let Some(f_obj) = file_info.as_object_mut() {
            f_obj.insert("hash".to_string(), Value::Null);
        }
        obj.insert("file".to_string(), file_info);
    }

    Ok(Value::Object(obj))
}

fn build_album(
    ctx: &AlbumContext,
    registry: &Map<String, Value>,
) -> Result<Value, VellumError> {
    let mut obj = serde_json::Map::new();

    obj.insert("albumartist".to_string(), resolvers::resolve_top_level_album_key("albumartist", ctx)?);
    obj.insert("album".to_string(), resolvers::resolve_top_level_album_key("album", ctx)?);
    obj.insert("comment".to_string(), resolvers::resolve_top_level_album_key("comment", ctx)?);
    obj.insert("date".to_string(), resolvers::resolve_top_level_album_key("date", ctx)?);
    obj.insert("original_date".to_string(), resolvers::resolve_top_level_album_key("original_date", ctx)?);
    obj.insert("release_date".to_string(), resolvers::resolve_top_level_album_key("release_date", ctx)?);
    obj.insert("genre".to_string(), resolvers::resolve_top_level_album_key("genre", ctx)?);
    
    let styles = libvellum::types::resolve_type_array(ctx.source, "styles", "", ctx.album_root)?;
    obj.insert("styles".to_string(), styles);

    let total_discs = libvellum::resolvers::calculate_total_discs(ctx.tracks);
    obj.insert("total_discs".to_string(), json!(total_discs));
    obj.insert("total_tracks".to_string(), json!(ctx.tracks.len()));

    obj.insert("id".to_string(), json!(libvellum::resolvers::rel_path(ctx.album_root, ctx.library_root)));

    let mut keys = serde_json::Map::new();
    for (key, meta) in registry {
        if meta.get("level").and_then(Value::as_str) != Some("album") {
            continue;
        }
        if ["album", "albumartist", "date", "genre", "comment", "original_date", "release_date", "styles"].contains(&key.as_str()) {
            continue;
        }
        let val = resolvers::resolve_album_key(key, meta, ctx)?.unwrap_or(Value::Null);
        keys.insert(key.clone(), val);
    }
    obj.insert("keys".to_string(), Value::Object(keys));

    let mut info = serde_json::Map::new();
    info.insert("date_added".to_string(), json!(libvellum::resolvers::resolve_album_info_date_added(ctx.album_root, ctx.source, ctx.config)?));
    
    let dur_ms: u64 = ctx.tracks.iter()
        .filter_map(|t| t.get("info").and_then(|i| i.get("duration_milliseconds")).and_then(Value::as_u64))
        .sum();
    info.insert("duration_milliseconds".to_string(), json!(dur_ms));
    info.insert("duration_formatted".to_string(), json!(libvellum::resolvers::format_ms(dur_ms)));
    
    obj.insert("info".to_string(), Value::Object(info));

    obj.insert("manifests".to_string(), Value::Array(ctx.manifests.clone()));
    obj.insert("covers".to_string(), ctx.covers.clone());

    Ok(Value::Object(obj))
}
