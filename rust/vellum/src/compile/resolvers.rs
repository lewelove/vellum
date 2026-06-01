pub mod cover_palette {
    pub use libvellum::images::cover_palette::process_image_to_palette;
    pub use libvellum::images::cover_palette::resolve_core;
}

use crate::compile::builder::context::{AlbumContext, TrackContext};
use serde_json::{Value, json};

pub fn resolve_top_level_album_key(key: &str, ctx: &AlbumContext) -> Value {
    match key {
        "album" => libvellum::types::resolve_type_string(ctx.source, "album", "", "Untitled"),
        "albumartist" => libvellum::types::resolve_type_string(ctx.source, "albumartist", "artistartist", "Unknown"),
        "date" => libvellum::types::resolve_type_string(ctx.source, "date", "year,originalyear", "0000"),
        "genre" => libvellum::resolvers::resolve_genre(ctx.source),
        "comment" => json!(libvellum::resolvers::resolve_comment(ctx.source)),
        "original_date" => json!(libvellum::resolvers::resolve_original_date(ctx.source)),
        "release_date" => json!(libvellum::resolvers::resolve_release_date(ctx.source)),
        _ => Value::Null,
    }
}

pub fn resolve_top_level_track_key(key: &str, ctx: &TrackContext) -> Value {
    match key {
        "title" => libvellum::types::resolve_type_string(ctx.source, "title", "", "Untitled"),
        "artist" => libvellum::resolvers::resolve_string_fallback(
            ctx.source, ctx.album_source, "artist", "albumartist", "Unknown"
        ),
        _ => Value::Null,
    }
}

pub fn resolve_album_key(key: &str, meta: &Value, ctx: &AlbumContext) -> Option<Value> {
    let class = meta.get("class").and_then(Value::as_str).unwrap_or("generic");
    let type_ = meta.get("type").and_then(Value::as_str).unwrap_or("string");
    let args = meta.get("args").and_then(Value::as_str).unwrap_or("");

    if class == "function" {
        let res = match key {
            "cover_chroma" => libvellum::resolvers::resolve_cover_chroma(ctx.cover_metrics),
            "cover_entropy" => libvellum::resolvers::resolve_cover_entropy(ctx.cover_metrics),
            "original_date" => Some(json!(libvellum::resolvers::resolve_original_date(ctx.source))),
            "release_date" => Some(json!(libvellum::resolvers::resolve_release_date(ctx.source))),
            "comment" => Some(json!(libvellum::resolvers::resolve_comment(ctx.source))),
            _ => {
                log::warn!("Native function for key '{key}' not found, falling back to generic.");
                None
            }
        };
        if res.is_some() {
            return res;
        }
    }

    if libvellum::types::get_raw_value(ctx.source, key, args).is_some() {
        match type_ {
            "datetime" => Some(libvellum::types::resolve_type_datetime(ctx.source, key, args)),
            "array" => Some(libvellum::types::resolve_type_array(ctx.source, key, args)),
            "integer" => Some(libvellum::types::resolve_type_integer(ctx.source, key, args)),
            "float" => Some(libvellum::types::resolve_type_float(ctx.source, key, args)),
            "boolean" => Some(libvellum::types::resolve_type_boolean(ctx.source, key, args)),
            "path" => Some(libvellum::types::resolve_type_path(ctx.source, key, args, ctx.album_root)),
            "url" => Some(libvellum::types::resolve_type_url(ctx.source, key, args)),
            "object" => Some(libvellum::types::resolve_type_object(ctx.source, key, args)),
            _ => Some(libvellum::types::resolve_type_string(ctx.source, key, args, "")),
        }
    } else {
        if let Some(first_track) = ctx.tracks.first()
            && let Some(tags) = first_track.get("tags")
            && let Some(t_val) = tags.get(key)
        {
            return Some(t_val.clone());
        }
        None
    }
}

pub fn resolve_track_key(key: &str, meta: &Value, ctx: &TrackContext) -> Option<Value> {
    let class = meta.get("class").and_then(Value::as_str).unwrap_or("generic");
    let type_ = meta.get("type").and_then(Value::as_str).unwrap_or("string");
    let args = meta.get("args").and_then(Value::as_str).unwrap_or("");

    if class == "function" {
        let res = {
            log::warn!("Native function for track key '{key}' not found, falling back to generic.");
            None
        };
        if res.is_some() {
            return res;
        }
    }

    let source = if libvellum::types::get_raw_value(ctx.source, key, args).is_some() {
        Some(ctx.source)
    } else if libvellum::types::get_raw_value(ctx.album_source, key, args).is_some() {
        Some(ctx.album_source)
    } else {
        None
    };

    source.map(|src| match type_ {
        "datetime" => libvellum::types::resolve_type_datetime(src, key, args),
        "array" => libvellum::types::resolve_type_array(src, key, args),
        "integer" => libvellum::types::resolve_type_integer(src, key, args),
        "float" => libvellum::types::resolve_type_float(src, key, args),
        "boolean" => libvellum::types::resolve_type_boolean(src, key, args),
        "path" => libvellum::types::resolve_type_path(src, key, args, ctx.album_root),
        "url" => libvellum::types::resolve_type_url(src, key, args),
        "object" => libvellum::types::resolve_type_object(src, key, args),
        _ => libvellum::types::resolve_type_string(src, key, args, ""),
    })
}
