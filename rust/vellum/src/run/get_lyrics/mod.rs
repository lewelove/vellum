pub mod api;
pub mod scraper;

use anyhow::{Context, Result};
use libvellum::config::AppConfig;
use libvellum::models::LockFile;
use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub async fn run(
    _config: &AppConfig,
    target_album: &Path,
    env_vars: &HashMap<String, String>,
) -> Result<()> {
    let api_key = env_vars
        .get("GENIUS_API_KEY")
        .cloned()
        .or_else(|| std::env::var("GENIUS_API_KEY").ok())
        .context("Genius API Key is required via GENIUS_API_KEY environment variable.")?;

    let lock_file_path = target_album.join("metadata.lock.json");
    if !lock_file_path.exists() {
        anyhow::bail!("metadata.lock.json not found in {}", target_album.display());
    }

    let lock_content = fs::read_to_string(&lock_file_path)?;
    let lock_data: LockFile = serde_json::from_str(&lock_content)?;

    let album_artist = &lock_data.album.albumartist;
    let total_discs = lock_data.album.info.total_discs;

    let genius_api = api::GeniusApi::new(api_key)?;
    let lyrics_scraper = scraper::Scraper::new()?;

    let lyrics_dir = target_album.join("Lyrics");
    fs::create_dir_all(&lyrics_dir)?;

    log::info!(
        "Fetching lyrics for: {} - {}",
        album_artist,
        lock_data.album.album
    );

    for track in &lock_data.tracks {
        let title = &track.title;
        let track_num = format!("{:02}", track.tracknumber);
        let disc_num = track.discnumber.to_string();

        let safe_title = scraper::sanitize_filename(title);

        let filename = if total_discs > 1 {
            format!("{disc_num}.{track_num} - {safe_title}.txt")
        } else {
            format!("{track_num} - {safe_title}.txt")
        };

        let dest_path = lyrics_dir.join(&filename);

        if dest_path.exists() {
            log::info!("  Skipping: {title} (File exists)");
            continue;
        }

        let query = format!("{album_artist} {title}");
        match genius_api.search(&query).await {
            Ok(Some(song)) => match lyrics_scraper.fetch_lyrics(&song.url).await {
                Ok(cleaned_text) => {
                    fs::write(&dest_path, cleaned_text)?;
                    log::info!("  Saved: {title}");
                }
                Err(e) => {
                    log::error!("  Error fetching lyrics for {title}: {e}");
                }
            },
            Ok(None) => {
                log::info!("  Not found: {title}");
            }
            Err(e) => {
                log::error!("  Error searching for {title}: {e}");
            }
        }
    }

    log::info!("get-lyrics completed successfully. Triggering library update...");
    crate::update::run(Some(target_album.to_path_buf()), false, None, false, false).await?;

    Ok(())
}
