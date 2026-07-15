use crate::api;
use crate::models::{AlbumData, FormattingConfig};
use anyhow::Result;
use serde::Serialize;
use std::fs;
use std::path::Path;
use std::str::FromStr;

#[derive(Serialize)]
struct MetadataToml {
    album: AlbumSection,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    tracks: Vec<TrackSection>,
}

#[derive(Serialize)]
struct AlbumSection {
    albumartist: String,
    album: String,
    date: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    genre: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    styles: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    discogs_url: Option<String>,
}

#[derive(Serialize)]
struct TrackSection {
    #[serde(skip_serializing_if = "Option::is_none")]
    discnumber: Option<u32>,
    tracknumber: u32,
    title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    artist: Option<String>,
}

#[derive(Serialize)]
struct LocalToml {
    local: LocalSection,
}

#[derive(Serialize)]
struct LocalSection {
    date_added: toml::value::Datetime,
    #[serde(rename = "virtual")]
    is_virtual: bool,
}

pub async fn create_album_directory(
    data: &AlbumData,
    formatting: &FormattingConfig,
    root: &Path,
) -> Result<()> {
    let formatted = formatting
        .album
        .replace("{albumartist}", &data.albumartist)
        .replace("{album}", &data.album);

    let dir_name = sanitize_filename(&formatted);
    let album_path = root.join(dir_name);

    fs::create_dir_all(&album_path)?;

    let info_path = album_path.join(&formatting.info);
    fs::create_dir_all(&info_path)?;

    if let Some(discogs) = &data.discogs_raw {
        let path = info_path.join("discogs_master.json");
        fs::write(path, serde_json::to_string_pretty(discogs)?)?;
    }

    let meta_path = album_path.join("metadata.toml");
    write_metadata_toml(data, &meta_path)?;

    let local_path = album_path.join("local.toml");
    write_local_toml(&local_path)?;

    let covers_dir = album_path.join("Digital Covers");

    if data.discogs_cover_url.is_some() {
        fs::create_dir_all(&covers_dir)?;
    }

    if let Some(url) = &data.discogs_cover_url {
        let ext = extract_extension(url);
        let dest = covers_dir.join(format!("discogs.{ext}"));
        if api::download_discogs_cover(url, &dest).await.is_ok() {
            let cover_root = album_path.join(format!("cover.{ext}"));
            fs::copy(&dest, cover_root).ok();
        }
    }

    Ok(())
}

fn sanitize_filename(name: &str) -> String {
    name.replace(&['/', '<', '>', ':', '"', '\\', '|', '?', '*'][..], "_")
}

fn extract_extension(url: &str) -> String {
    let without_query = url.split('?').next().unwrap_or(url);
    let ext = without_query.split('.').next_back().unwrap_or("jpg").to_lowercase();
    if ext == "png" {
        "png".to_string()
    } else {
        "jpg".to_string()
    }
}

fn write_metadata_toml(data: &AlbumData, path: &Path) -> Result<()> {
    let total_discs = data.tracks.iter().map(|t| t.discnumber).max().unwrap_or(1);

    let mut track_sections = Vec::new();
    for t in &data.tracks {
        let mut track_artist = None;
        if let Some(art) = &t.artist
            && art != &data.albumartist
        {
            track_artist = Some(art.clone());
        }

        track_sections.push(TrackSection {
            discnumber: if total_discs > 1 { Some(t.discnumber) } else { None },
            tracknumber: t.tracknumber,
            title: t.title.clone(),
            artist: track_artist,
        });
    }

    let meta = MetadataToml {
        album: AlbumSection {
            albumartist: data.albumartist.clone(),
            album: data.album.clone(),
            date: data.date.clone(),
            genre: data.genre.clone(),
            styles: data.styles.clone(),
            discogs_url: data.discogs_url.clone(),
        },
        tracks: track_sections,
    };

    let content = toml::to_string(&meta)?;
    fs::write(path, content)?;
    Ok(())
}

fn write_local_toml(path: &Path) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let dt = toml::value::Datetime::from_str(&now)?;

    let local_meta = LocalToml {
        local: LocalSection { 
            date_added: dt,
            is_virtual: true,
        },
    };

    let content = toml::to_string(&local_meta)?;
    fs::write(path, content)?;
    Ok(())
}
