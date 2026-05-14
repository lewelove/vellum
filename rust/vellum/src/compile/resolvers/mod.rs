pub mod native;
pub mod standard;
pub mod cover_palette;

use crate::compile::builder::context::{AlbumContext, TrackContext};
use serde_json::{Value, json};

pub fn resolve_top_level_album_key(key: &str, ctx: &AlbumContext) -> Value {
    match key {
        "album" => standard::resolve_generic_string(ctx.source, "album", "", "Untitled"),
        "albumartist" => standard::resolve_generic_string(ctx.source, "albumartist", "artistartist", "Unknown"),
        "date" => standard::resolve_generic_string(ctx.source, "date", "year,originalyear", "0000"),
        "genre" => {
            let mut list = standard::resolve_generic_list(ctx.source, "genre", "");
            if let Value::Array(ref arr) = list
                && arr.is_empty() {
                    list = json!(["Unknown"]);
                }
            list
        },
        "comment" => json!(native::resolve_comment(ctx, "")),
        "original_date" => json!(native::resolve_date(ctx, "original_date", "")),
        "release_date" => json!(native::resolve_date(ctx, "release_date", "")),
        _ => Value::Null,
    }
}

pub fn resolve_top_level_track_key(key: &str, ctx: &TrackContext) -> Value {
    match key {
        "title" => standard::resolve_generic_string(ctx.source, "title", "", "Untitled"),
        "artist" => standard::resolve_generic_string_fallback(
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
            "cover_chroma" => native::resolve_cover_chroma(ctx, args),
            "cover_entropy" => native::resolve_cover_entropy(ctx, args),
            "original_date" => Some(json!(native::resolve_date(ctx, "original_date", args))),
            "release_date" => Some(json!(native::resolve_date(ctx, "release_date", args))),
            "comment" => Some(json!(native::resolve_comment(ctx, args))),
            _ => {
                log::warn!("Native function for key '{key}' not found, falling back to generic.");
                None
            }
        };
        if res.is_some() {
            return res;
        }
    }

    if standard::get_raw_value(ctx.source, key, args).is_some() {
        match type_ {
            "time" => Some(standard::resolve_generic_time(ctx.source, key, args)),
            "list" => Some(standard::resolve_generic_list(ctx.source, key, args)),
            "integer" => Some(standard::resolve_generic_integer(ctx.source, key, args)),
            "float" => Some(standard::resolve_generic_float(ctx.source, key, args)),
            "bool" => Some(standard::resolve_generic_bool(ctx.source, key, args)),
            _ => Some(standard::resolve_generic_string(ctx.source, key, args, "")),
        }
    } else {
        if let Some(first_track) = ctx.tracks.first()
            && let Some(tags) = first_track.get("tags")
                && let Some(t_val) = tags.get(key) {
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

    let source = if standard::get_raw_value(ctx.source, key, args).is_some() {
        Some(ctx.source)
    } else if standard::get_raw_value(ctx.album_source, key, args).is_some() {
        Some(ctx.album_source)
    } else {
        None
    };

    source.map(|src| match type_ {
        "time" => standard::resolve_generic_time(src, key, args),
        "list" => standard::resolve_generic_list(src, key, args),
        "integer" => standard::resolve_generic_integer(src, key, args),
        "float" => standard::resolve_generic_float(src, key, args),
        "bool" => standard::resolve_generic_bool(src, key, args),
        _ => standard::resolve_generic_string(src, key, args, ""),
    })
}
