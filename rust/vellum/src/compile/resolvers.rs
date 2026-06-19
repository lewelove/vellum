use crate::compile::builder::context::{AlbumContext, TrackContext};
use serde_json::{Value, json};
use libvellum::error::VellumError;

pub fn resolve_top_level_album_key(key: &str, ctx: &AlbumContext) -> Result<Value, VellumError> {
    match key {
        "album" => libvellum::types::resolve_type_string(ctx.source, "album", "", "Untitled", ctx.album_root),
        "albumartist" => libvellum::types::resolve_type_string(ctx.source, "albumartist", "artistartist", "Unknown", ctx.album_root),
        "date" => libvellum::types::resolve_type_string(ctx.source, "date", "year,originalyear", "0000", ctx.album_root),
        "genre" => libvellum::resolvers::resolve_genre(ctx.source, ctx.album_root),
        "comment" => Ok(json!(libvellum::resolvers::resolve_comment(ctx.source, ctx.album_root)?)),
        "original_date" => Ok(json!(libvellum::resolvers::resolve_original_date(ctx.source, ctx.album_root)?)),
        "release_date" => Ok(json!(libvellum::resolvers::resolve_release_date(ctx.source, ctx.album_root)?)),
        _ => Ok(Value::Null),
    }
}

pub fn resolve_top_level_track_key(key: &str, ctx: &TrackContext) -> Result<Value, VellumError> {
    match key {
        "title" => libvellum::types::resolve_type_string(ctx.source, "title", "", "Untitled", ctx.album_root),
        "artist" => Ok(libvellum::resolvers::resolve_string_fallback(
            ctx.source, ctx.album_source, "artist", "albumartist", "Unknown"
        )),
        _ => Ok(Value::Null),
    }
}

pub fn resolve_album_key(key: &str, meta: &Value, ctx: &AlbumContext) -> Result<Option<Value>, VellumError> {
    let type_ = meta.get("type").and_then(Value::as_str).unwrap_or("string");
    let args = meta.get("args").and_then(Value::as_str).unwrap_or("");

    if libvellum::types::get_raw_value(ctx.source, key, args).is_some() || type_ == "function" {
        match type_ {
            "function" => Ok(Some(libvellum::types::resolve_type_function(key, ctx.source, ctx.cover_metrics, ctx.album_root)?)),
            "datetime" => Ok(Some(libvellum::types::resolve_type_datetime(ctx.source, key, args, ctx.album_root)?)),
            "array" => Ok(Some(libvellum::types::resolve_type_array(ctx.source, key, args, ctx.album_root)?)),
            "integer" => Ok(Some(libvellum::types::resolve_type_integer(ctx.source, key, args, ctx.album_root)?)),
            "float" => Ok(Some(libvellum::types::resolve_type_float(ctx.source, key, args, ctx.album_root)?)),
            "boolean" => Ok(Some(libvellum::types::resolve_type_boolean(ctx.source, key, args, ctx.album_root)?)),
            "path" => Ok(Some(libvellum::types::resolve_type_path(ctx.source, key, args, ctx.album_root)?)),
            "url" => Ok(Some(libvellum::types::resolve_type_url(ctx.source, key, args, ctx.album_root)?)),
            "object" => Ok(Some(libvellum::types::resolve_type_object(ctx.source, key, args, ctx.album_root)?)),
            _ => Ok(Some(libvellum::types::resolve_type_string(ctx.source, key, args, "", ctx.album_root)?)),
        }
    } else {
        if let Some(first_track) = ctx.tracks.first()
            && let Some(tags) = first_track.get("tags")
            && let Some(t_val) = tags.get(key)
        {
            return Ok(Some(t_val.clone()));
        }
        Ok(None)
    }
}

pub fn resolve_track_key(key: &str, meta: &Value, ctx: &TrackContext) -> Result<Option<Value>, VellumError> {
    let type_ = meta.get("type").and_then(Value::as_str).unwrap_or("string");
    let args = meta.get("args").and_then(Value::as_str).unwrap_or("");

    let source = if libvellum::types::get_raw_value(ctx.source, key, args).is_some() {
        Some(ctx.source)
    } else if libvellum::types::get_raw_value(ctx.album_source, key, args).is_some() {
        Some(ctx.album_source)
    } else {
        None
    };

    if source.is_some() || type_ == "function" {
        let src = source.unwrap_or(&Value::Null);
        match type_ {
            "function" => Ok(Some(libvellum::types::resolve_type_function(key, src, None, ctx.album_root)?)),
            "datetime" => Ok(Some(libvellum::types::resolve_type_datetime(src, key, args, ctx.album_root)?)),
            "array" => Ok(Some(libvellum::types::resolve_type_array(src, key, args, ctx.album_root)?)),
            "integer" => Ok(Some(libvellum::types::resolve_type_integer(src, key, args, ctx.album_root)?)),
            "float" => Ok(Some(libvellum::types::resolve_type_float(src, key, args, ctx.album_root)?)),
            "boolean" => Ok(Some(libvellum::types::resolve_type_boolean(src, key, args, ctx.album_root)?)),
            "path" => Ok(Some(libvellum::types::resolve_type_path(src, key, args, ctx.album_root)?)),
            "url" => Ok(Some(libvellum::types::resolve_type_url(src, key, args, ctx.album_root)?)),
            "object" => Ok(Some(libvellum::types::resolve_type_object(src, key, args, ctx.album_root)?)),
            _ => Ok(Some(libvellum::types::resolve_type_string(src, key, args, "", ctx.album_root)?)),
        }
    } else {
        Ok(None)
    }
}
