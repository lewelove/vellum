use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverMetrics {
    pub hash: String,
    pub entropy: Option<usize>,
    pub chroma: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HashInfo {
    pub string: String,
    pub address: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FileInfo {
    pub path: String,
    pub hash: Option<HashInfo>,
    pub mtime: u64,
    pub byte_size: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestEntry {
    pub file: FileInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverMain {
    pub file: FileInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoversEntry {
    pub main: CoverMain,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlbumInfo {
    #[serde(default)]
    pub date_added: String,
    #[serde(default)]
    pub duration_milliseconds: u64,
    #[serde(default)]
    pub duration_formatted: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlbumLock {
    pub albumartist: String,
    pub album: String,
    #[serde(default)]
    pub comment: String,
    #[serde(default)]
    pub date: String,
    #[serde(default)]
    pub original_date: String,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub genre: Vec<String>,
    #[serde(default)]
    pub styles: Vec<String>,
    pub total_discs: u32,
    pub total_tracks: u32,
    pub id: String,
    #[serde(default)]
    pub keys: HashMap<String, serde_json::Value>,
    pub info: AlbumInfo,
    #[serde(default)]
    pub manifests: Vec<ManifestEntry>,
    pub covers: Option<CoversEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LyricsEntry {
    pub file: FileInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackInfo {
    #[serde(default)]
    pub sample_rate: u32,
    #[serde(default)]
    pub bits_per_sample: u8,
    #[serde(default)]
    pub bitrate_kbps: u32,
    #[serde(default)]
    pub encoding: String,
    #[serde(default)]
    pub channels: u8,
    #[serde(default)]
    pub duration_milliseconds: u64,
    #[serde(default)]
    pub duration_formatted: String,
    #[serde(default)]
    pub embedded_keys_subset_match: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackLock {
    pub discnumber: u32,
    pub tracknumber: u32,
    pub artist: String,
    pub title: String,
    pub lyrics: Option<LyricsEntry>,
    #[serde(default)]
    pub keys: HashMap<String, serde_json::Value>,
    pub info: TrackInfo,
    pub file: FileInfo,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LockFile {
    pub album: AlbumLock,
    pub tracks: Vec<TrackLock>,
}
