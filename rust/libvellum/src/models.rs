use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CoverMetrics {
    pub hash: String,
    pub entropy: Option<usize>,
    pub chroma: Option<f64>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackInfo {
    #[serde(default)]
    pub track_path: String,
    #[serde(default)]
    pub track_library_path: String,
    #[serde(default)]
    pub track_duration: u64,
    #[serde(default)]
    pub track_duration_time: String,
    #[serde(default)]
    pub encoding: String,
    #[serde(default)]
    pub sample_rate: u32,
    #[serde(default)]
    pub bits_per_sample: u8,
    #[serde(default)]
    pub channels: u8,
    #[serde(default)]
    pub track_mtime: u64,
    #[serde(default)]
    pub track_byte_size: u64,
    #[serde(default)]
    pub lyrics_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackLock {
    pub info: TrackInfo,
    #[serde(rename = "title")]
    pub title: String,
    #[serde(rename = "artist")]
    pub artist: String,
    #[serde(rename = "tracknumber")]
    pub tracknumber: u32,
    #[serde(rename = "discnumber")]
    pub discnumber: u32,
    #[serde(default)]
    pub tags: HashMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlbumInfo {
    #[serde(default)]
    pub album_path: String,
    #[serde(default)]
    pub date_added: String,
    #[serde(default)]
    pub album_duration: u64,
    #[serde(default)]
    pub album_duration_time: String,
    #[serde(default)]
    pub total_discs: u32,
    #[serde(default)]
    pub total_tracks: u32,
    #[serde(default)]
    pub metadata_toml_hash: String,
    #[serde(default)]
    pub metadata_toml_mtime: u64,
    #[serde(default)]
    pub manifests_mtime_sum: u64,
    #[serde(default)]
    pub file_tag_subset_match: bool,
    #[serde(default)]
    pub cover_path: String,
    #[serde(default)]
    pub cover_hash: String,
    #[serde(default)]
    pub cover_mtime: u64,
    #[serde(default)]
    pub cover_byte_size: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AlbumLock {
    pub info: AlbumInfo,
    #[serde(rename = "album")]
    pub album: String,
    #[serde(rename = "albumartist")]
    pub albumartist: String,
    #[serde(rename = "date")]
    pub date: String,
    #[serde(
        rename = "genre",
        default,
        deserialize_with = "deserialize_vec_or_string"
    )]
    pub genre: Vec<String>,
    #[serde(rename = "comment", default)]
    pub comment: String,
    #[serde(rename = "original_date", default)]
    pub original_date: String,
    #[serde(rename = "original_year", default)]
    pub original_year: String,
    #[serde(rename = "release_date", default)]
    pub release_date: String,
    #[serde(rename = "release_year", default)]
    pub release_year: String,
    #[serde(default)]
    pub tags: HashMap<String, serde_json::Value>,
}

fn deserialize_vec_or_string<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum VecOrString {
        Vec(Vec<String>),
        String(String),
    }

    match VecOrString::deserialize(deserializer)? {
        VecOrString::Vec(v) => Ok(v),
        VecOrString::String(s) => Ok(s
            .split(';')
            .map(|part| part.trim().to_string())
            .filter(|part| !part.is_empty())
            .collect()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct LockFile {
    pub album: AlbumLock,
    pub tracks: Vec<TrackLock>,
}
