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
    #[serde(default)]
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub address: Option<String>,
    #[serde(default)]
    pub mtime: u64,
    #[serde(default)]
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
    pub total_discs: u32,
    pub total_tracks: u32,
    #[serde(default)]
    pub date_added: String,
    #[serde(default)]
    pub duration_milliseconds: u64,
    #[serde(default)]
    pub duration_formatted: String,
    #[serde(default, rename = "virtual")]
    pub is_virtual: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlbumLock {
    pub albumartist: String,
    pub album: String,
    pub date: String,
    pub id: String,
    #[serde(default)]
    pub keys: HashMap<String, serde_json::Value>,
    pub info: AlbumInfo,
    #[serde(default)]
    pub manifests: HashMap<String, ManifestEntry>,
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
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackLock {
    pub discnumber: u32,
    pub tracknumber: u32,
    pub artist: String,
    pub title: String,
    #[serde(default)]
    pub keys: HashMap<String, serde_json::Value>,
    pub info: TrackInfo,
    pub file: FileInfo,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lyrics: Option<LyricsEntry>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LockFile {
    pub album: AlbumLock,
    pub tracks: Vec<TrackLock>,
}
