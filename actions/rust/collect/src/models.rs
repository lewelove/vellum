use serde::Deserialize;

#[derive(Deserialize)]
pub struct ActionPayload {
    pub config: ConfigBlock,
    pub options: String,
}

#[derive(Deserialize)]
pub struct ConfigBlock {
    pub action: ActionConfig,
}

#[derive(Deserialize)]
pub struct ActionConfig {
    pub root: String,
    pub formatting: FormattingConfig,
}

#[derive(Deserialize)]
pub struct FormattingConfig {
    pub album: String,
    pub info: String,
}

pub enum TargetUrl {
    DiscogsMaster(u64),
    DiscogsRelease(u64),
}

pub struct Track {
    pub discnumber: u32,
    pub tracknumber: u32,
    pub title: String,
    pub artist: Option<String>,
}

#[derive(Default)]
pub struct AlbumData {
    pub albumartist: String,
    pub album: String,
    pub date: String,
    pub genre: Vec<String>,
    pub styles: Vec<String>,
    pub discogs_master_url: Option<String>,
    pub tracks: Vec<Track>,
    pub discogs_raw: Option<serde_json::Value>,
    pub discogs_cover_url: Option<String>,
}
