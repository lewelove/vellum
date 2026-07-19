use crate::compile::{album, context, covers, tracks, utils};
use libvellum::compiler::manifest::load_manifests;
use libvellum::error::VellumError;
use serde::de::Error as _;
use serde_json::{Value, json};
use std::path::Path;

pub fn build(
    album_root: &Path,
    config: &libvellum::lua::ResolvedConfig,
) -> Result<Value, VellumError> {
    let manifest_names = config.app.compiler.manifests.as_ref().map(|v| v.iter().map(|s| Value::String(s.clone())).collect::<Vec<_>>());
    let parsed_manifests = load_manifests(album_root, manifest_names.as_ref())?;

    let primary_manifest = parsed_manifests.get("metadata").ok_or_else(|| VellumError::MissingPrimaryManifest { path: album_root.to_path_buf() })?;
    let primary_tracks = primary_manifest.get("tracks").and_then(Value::as_array).ok_or_else(|| VellumError::MissingTracksBlock { path: album_root.to_path_buf() })?;
    
    let (cover_file_info, cover_metrics) = covers::resolve_cover_data(album_root, config);

    let context::PreparedContext { audio_files, library_root } = context::prepare_build_context(config, album_root);

    let is_virtual = album::is_virtual_album(album_root);

    tracks::validate_audio_files(is_virtual, &audio_files, primary_tracks, album_root)?;

    let lock_manifests = album::generate_lock_manifests(&parsed_manifests, album_root);

    let total_discs = libvellum::resolvers::calculate_total_discs(primary_tracks);
    let total_tracks = primary_tracks.len() as u32;

    let (ctx_tracks, duration_sum_ms) = tracks::build_ctx_tracks(is_virtual, primary_tracks, &audio_files, album_root)?;

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

    let mut album_keys = lua_res.get("album").cloned().unwrap_or_else(|| json!({}));
    utils::sort_json_keys(&mut album_keys);

    let track_keys_array = lua_res.get("tracks").and_then(Value::as_array);

    let empty_album = json!({});
    let primary_album = primary_manifest.get("album").unwrap_or(&empty_album);
    
    let (albumartist, album_title, date) = album::parse_mandatory_album_fields(primary_album, album_root)?;

    let date_added = libvellum::resolvers::resolve_album_info_date_added(album_root, primary_album, config)?;

    let info_obj = json!({
        "virtual": is_virtual,
        "total_discs": total_discs,
        "total_tracks": total_tracks,
        "date_added": date_added,
        "duration_milliseconds": duration_sum_ms,
        "duration_formatted": libvellum::resolvers::format_ms(duration_sum_ms),
    });

    let final_tracks = tracks::build_final_tracks(
        primary_tracks,
        &albumartist,
        track_keys_array,
        &ctx_tracks,
        album_root,
        total_discs,
    )?;

    let mut album_obj = serde_json::Map::new();
    album_obj.insert("albumartist".to_string(), json!(albumartist));
    album_obj.insert("album".to_string(), json!(album_title));
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

    let mut final_json = serde_json::Map::new();
    final_json.insert("album".to_string(), Value::Object(album_obj));
    final_json.insert("tracks".to_string(), Value::Array(final_tracks));
    final_json.insert("ctx".to_string(), json!({
        "paths": {
            "album_root": album_root.to_string_lossy(),
            "project_root": config.path.parent().unwrap_or_else(|| Path::new(".")).to_string_lossy(),
            "library_root": library_root.to_string_lossy()
        }
    }));

    Ok(Value::Object(final_json))
}
