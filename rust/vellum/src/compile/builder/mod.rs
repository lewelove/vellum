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

    let (c_path, c_hash, c_mtime, c_size) = assets::resolve_cover_info(album_root);
    let loaded_image =
        assets::load_or_create_thumbnail(config, album_root, c_path.as_deref(), &c_hash);

    let cover_metrics = resolve_cover_metrics(config, &c_hash, loaded_image.as_ref(), &manifest_data);

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

    let empty_obj = json!({});
    let album_source = manifest_data.json.get("album").unwrap_or(&empty_obj);

    validate_album_level_keys(album_source, track_entries, &registry, album_root)?;

    let (final_tracks, harvested_cache) = process_tracks(
        audio_files,
        track_entries,
        album_source,
        album_root,
        &library_root,
        &registry,
    )?;

    let album_ctx = AlbumContext {
        source: album_source,
        tracks: &final_tracks,
        album_root,
        library_root: &library_root,
        meta_hash: &manifest_data.meta_hash,
        meta_mtime: manifest_data.meta_mtime,
        manifests_mtime_sum: manifest_data.manifests_mtime_sum,
        cover_hash: &c_hash,
        cover_path: c_path.as_deref(),
        cover_mtime: c_mtime,
        cover_byte_size: c_size,
        cover_metrics: cover_metrics.as_ref(),
        config,
    };

    let album_obj = build_album(&album_ctx, &registry);

    let final_json = json!({
        "album": album_obj,
        "tracks": final_tracks,
        "ctx": {
            "config": config,
            "registry": registry,
            "metadata": manifest_data.json,
            "harvest": harvested_cache,
            "paths": {
                "album_root": album_root.to_string_lossy(),
                "project_root": project_root.to_string_lossy(),
                "library_root": library_root.to_string_lossy()
            }
        }
    });

    Ok(final_json)
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

    let mut registry = config
        .get("compiler")
        .and_then(|c| c.get("keys"))
        .and_then(Value::as_object)
        .ok_or_else(|| VellumError::MissingCompilerRegistry)?
        .clone();

    merge_local_registry(album_root, &mut registry);

    Ok(PreparedContext { audio_files, registry, library_root })
}

fn resolve_cover_metrics(
    config: &Value,
    c_hash: &str,
    loaded_image: Option<&image::DynamicImage>,
    manifest_data: &libvellum::compiler::manifest::ManifestData,
) -> Option<assets::CoverMetrics> {
    if c_hash.is_empty() {
        return None;
    }
    
    let cache_str = config.get("storage").and_then(|s| s.get("cache")).and_then(Value::as_str).unwrap_or("~/.cache/vellum");
    let cache_root = crate::expand_path(cache_str);
    let metrics_dir = cache_root.join("cover_data");
    std::fs::create_dir_all(&metrics_dir).ok();
    
    let metrics_path = metrics_dir.join(format!("{c_hash}.json"));
    
    let palette_cfg = config.get("compiler").and_then(|c| c.get("cover_palette"));
    let cover_palette_raw = manifest_data.json.get("album").and_then(|a| a.get("cover_palette"));
    
    let palette_params = format!("{palette_cfg:?}|{cover_palette_raw:?}");
    
    let mut metrics = if metrics_path.exists() {
        std::fs::read_to_string(&metrics_path).map_or(None, |content| serde_json::from_str::<assets::CoverMetrics>(&content).ok())
    } else { 
        None 
    }.unwrap_or_else(|| assets::CoverMetrics {
        hash: c_hash.to_string(),
        entropy: None,
        chroma: None,
        palette: None,
        palette_params: None,
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
        
        if (metrics.palette_params.as_deref() != Some(&palette_params) || metrics.palette.is_none())
            && let Some(palette_val) = resolvers::cover_palette::resolve_core(img, palette_cfg, cover_palette_raw) {
                metrics.palette = Some(palette_val);
                metrics.palette_params = Some(palette_params);
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
    library_root: &Path,
    registry: &Map<String, Value>,
) -> Result<(Vec<Value>, Vec<Value>), VellumError> {
    let mut harvested_spine = Vec::new();
    for path in audio_files {
        harvested_spine.push(harvest::harvest_file(&path).map_err(|source| VellumError::HarvestError { path: path.clone(), source })?);
    }

    let mut total_discs = 1;
    for t in track_entries {
        if let Ok(d) = extract_strict_u32(t.get("discnumber"), "discnumber", Some(1))
            && d > total_discs {
                total_discs = d;
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
            library_root,
        };

        let t_obj = build_track(&t_ctx, total_discs, registry);
        final_tracks.push(t_obj);
        harvested_cache.push(serde_json::to_value(h_data)?);
    }

    Ok((final_tracks, harvested_cache))
}

fn construct_track_info(ctx: &TrackContext, total_discs: u32) -> Value {
    let mut info = serde_json::Map::new();

    let lyrics_path = ctx
        .source
        .get("lyrics_path")
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .or_else(|| {
            resolvers::native::resolve_lyrics_path(
                ctx.album_root,
                ctx.track_number,
                ctx.disc_number,
                total_discs,
            )
        });

    info.insert(
        "track_path".to_string(),
        json!(resolvers::native::rel_path(
            &ctx.harvest.path,
            ctx.album_root
        )),
    );
    info.insert(
        "track_library_path".to_string(),
        json!(resolvers::native::rel_path(
            &ctx.harvest.path,
            ctx.library_root
        )),
    );
    info.insert(
        "track_duration".to_string(),
        json!(ctx.harvest.physics.duration_ms),
    );
    info.insert(
        "track_duration_time".to_string(),
        json!(resolvers::standard::format_ms(
            ctx.harvest.physics.duration_ms
        )),
    );
    info.insert("encoding".to_string(), json!(ctx.harvest.physics.format));
    info.insert(
        "sample_rate".to_string(),
        json!(ctx.harvest.physics.sample_rate),
    );
    info.insert(
        "bits_per_sample".to_string(),
        json!(ctx.harvest.physics.bit_depth.unwrap_or(0)),
    );
    info.insert("channels".to_string(), json!(ctx.harvest.physics.channels));
    info.insert("track_mtime".to_string(), json!(ctx.harvest.physics.mtime));
    info.insert(
        "track_size".to_string(),
        json!(ctx.harvest.physics.file_size),
    );
    info.insert(
        "lyrics_path".to_string(),
        json!(lyrics_path.unwrap_or_default()),
    );

    Value::Object(info)
}

fn build_track(
    ctx: &TrackContext,
    total_discs: u32,
    registry: &Map<String, Value>,
) -> Value {
    let mut obj = serde_json::Map::new();

    obj.insert("info".to_string(), construct_track_info(ctx, total_discs));

    obj.insert("title".to_string(), resolvers::resolve_top_level_track_key("title", ctx));
    obj.insert("artist".to_string(), resolvers::resolve_top_level_track_key("artist", ctx));
    obj.insert("tracknumber".to_string(), json!(ctx.track_number));
    obj.insert("discnumber".to_string(), json!(ctx.disc_number));

    let mut tags = serde_json::Map::new();
    for (key, meta) in registry {
        let level = meta.get("level").and_then(Value::as_str).unwrap_or("");
        if level != "tracks" && level != "track" {
            continue;
        }

        if["title", "artist", "tracknumber", "discnumber"].contains(&key.as_str()) {
            continue;
        }
        let val = resolvers::resolve_track_key(key, meta, ctx).unwrap_or(Value::Null);
        tags.insert(key.clone(), val);
    }
    obj.insert("tags".to_string(), Value::Object(tags));
    Value::Object(obj)
}

fn construct_album_info(ctx: &AlbumContext) -> Value {
    let mut info = serde_json::Map::new();
    let dur: u64 = ctx
        .tracks
        .iter()
        .filter_map(|t| {
            t.get("info")
                .and_then(|i| i.get("track_duration"))
                .and_then(Value::as_u64)
        })
        .sum();

    info.insert(
        "album_path".to_string(),
        json!(resolvers::native::rel_path(
            ctx.album_root,
            ctx.library_root
        )),
    );
    info.insert(
        "date_added".to_string(),
        json!(resolvers::native::resolve_album_info_date_added(ctx, "")),
    );
    info.insert("album_duration".to_string(), json!(dur));
    info.insert(
        "album_duration_time".to_string(),
        json!(resolvers::standard::format_ms(dur)),
    );
    info.insert(
        "total_discs".to_string(),
        json!(resolvers::native::calculate_total_discs(ctx.tracks)),
    );
    info.insert("total_tracks".to_string(), json!(ctx.tracks.len()));
    info.insert("metadata_toml_hash".to_string(), json!(ctx.meta_hash));
    info.insert("metadata_toml_mtime".to_string(), json!(ctx.meta_mtime));
    info.insert("manifests_mtime_sum".to_string(), json!(ctx.manifests_mtime_sum));
    info.insert(
        "cover_path".to_string(),
        json!(ctx.cover_path.unwrap_or("default_cover.png")),
    );
    info.insert("cover_hash".to_string(), json!(ctx.cover_hash));
    info.insert("cover_mtime".to_string(), json!(ctx.cover_mtime));
    info.insert("cover_byte_size".to_string(), json!(ctx.cover_byte_size));

    Value::Object(info)
}

fn build_album(
    ctx: &AlbumContext,
    registry: &Map<String, Value>,
) -> Value {
    let mut obj = serde_json::Map::new();

    obj.insert("info".to_string(), construct_album_info(ctx));
    obj.insert("album".to_string(), resolvers::resolve_top_level_album_key("album", ctx));
    obj.insert("albumartist".to_string(), resolvers::resolve_top_level_album_key("albumartist", ctx));
    obj.insert("date".to_string(), resolvers::resolve_top_level_album_key("date", ctx));
    obj.insert("genre".to_string(), resolvers::resolve_top_level_album_key("genre", ctx));
    obj.insert("comment".to_string(), resolvers::resolve_top_level_album_key("comment", ctx));
    obj.insert("original_date".to_string(), resolvers::resolve_top_level_album_key("original_date", ctx));
    obj.insert("release_date".to_string(), resolvers::resolve_top_level_album_key("release_date", ctx));

    let mut tags = serde_json::Map::new();
    for (key, meta) in registry {
        if meta.get("level").and_then(Value::as_str) != Some("album") {
            continue;
        }

        if["album", "albumartist", "date", "genre", "comment", "original_date", "release_date"].contains(&key.as_str()) {
            continue;
        }
        let val = resolvers::resolve_album_key(key, meta, ctx).unwrap_or(Value::Null);
        tags.insert(key.clone(), val);
    }

    if let Some(palette_cfg) = ctx.config.get("compiler").and_then(|c| c.get("cover_palette"))
        && let Some(val) = resolvers::native::resolve_cover_palette(ctx, palette_cfg) {
            tags.insert("cover_palette".to_string(), val);
        }

    obj.insert("tags".to_string(), Value::Object(tags));
    Value::Object(obj)
}
