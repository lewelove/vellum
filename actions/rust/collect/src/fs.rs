use crate::api;
use crate::models::{AlbumData, FormattingConfig};
use anyhow::Result;
use std::fs;
use std::path::Path;

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

    let system_path = album_path.join("system.toml");
    write_system_toml(&system_path)?;

    let covers_dir = album_path.join("Digital Covers");

    if data.discogs_cover_url.is_some() {
        fs::create_dir_all(&covers_dir)?;
    }

    if let Some(url) = &data.discogs_cover_url {
        let ext = extract_extension(url);
        let cover_root = album_path.join(format!("cover.{ext}"));
        if let Err(e) = api::download_discogs_cover(url, &cover_root).await {
            eprintln!("Failed to download cover: {e:?}");
        }
    } else {
        eprintln!("No discogs cover URL found in album data.");
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

fn escape_toml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn toml_array(arr: &[String]) -> String {
    let escaped: Vec<String> = arr
        .iter()
        .map(|s| format!("\"{}\"", escape_toml_string(s)))
        .collect();
    format!("[{}]", escaped.join(", "))
}

fn write_metadata_toml(data: &AlbumData, path: &Path) -> Result<()> {
    let mut lines = vec![
        "[album]".to_string(),
        String::new(),
        format!("albumartist = \"{}\"", escape_toml_string(&data.albumartist)),
        format!("album = \"{}\"", escape_toml_string(&data.album)),
        format!("date = \"{}\"", escape_toml_string(&data.date)),
        String::new(),
    ];

    if !data.genre.is_empty() {
        lines.push("genre = \"\"".to_string());
        lines.push(format!("genres = {}", toml_array(&data.genre)));
    }
    if !data.styles.is_empty() {
        lines.push(format!("styles = {}", toml_array(&data.styles)));
    }
    if let Some(ref durl) = data.discogs_master_url {
        lines.push(String::new());
        lines.push(format!("discogs_master_url = \"{}\"", escape_toml_string(durl)));
    }

    lines.push(String::new());

    let total_discs = data.tracks.iter().map(|t| t.discnumber).max().unwrap_or(1);

    for t in &data.tracks {
        lines.push("[[tracks]]".to_string());
        if total_discs > 1 {
            lines.push(format!("discnumber = {}", t.discnumber));
        }
        lines.push(format!("tracknumber = {}", t.tracknumber));
        lines.push(format!("title = \"{}\"", escape_toml_string(&t.title)));

        if let Some(ref art) = t.artist
            && art != &data.albumartist
        {
            lines.push(format!("artist = \"{}\"", escape_toml_string(art)));
        }
        lines.push(String::new());
    }

    fs::write(path, lines.join("\n"))?;
    Ok(())
}

fn write_system_toml(path: &Path) -> Result<()> {
    let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    let content = format!("[album.system]\n\ndate_generated = {now}\nvirtual = true\n");
    fs::write(path, content)?;
    Ok(())
}
