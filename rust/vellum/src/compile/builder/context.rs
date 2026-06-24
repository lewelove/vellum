use libvellum::models::CoverMetrics;
use crate::harvest::TrackJson;
use serde_json::Value;
use std::path::Path;

pub struct AlbumContext<'a> {
    pub source: &'a Value,
    pub tracks: &'a [Value],
    pub album_root: &'a Path,
    pub library_root: &'a Path,
    pub cover_metrics: Option<&'a CoverMetrics>,
    pub config: &'a Value,
    pub manifests: Vec<Value>,
    pub covers: Value,
}

pub struct TrackContext<'a> {
    pub track_number: u32,
    pub disc_number: u32,
    pub harvest: &'a TrackJson,
    pub source: &'a Value,
    pub album_source: &'a Value,
    pub album_root: &'a Path,
}
