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
