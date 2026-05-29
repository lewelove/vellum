use crate::server::query::QueryEngine;
use anyhow::{Context, Result};
use mpd_client::Client;
use mpd_client::commands;
use mpd_client::responses::PlayState;
use std::sync::Arc;
use tokio::sync::{Mutex, broadcast};

pub async fn broadcast_status(
    client: &Client,
    tx: &broadcast::Sender<String>,
    query: &Arc<Mutex<QueryEngine>>,
) -> Result<()> {
    let (status, current_song, queue) = client
        .command_list((commands::Status, commands::CurrentSong, commands::Queue))
        .await
        .context("Batched status update failed")?;

    let (file_path, title, artist) = if let Some(s) = current_song {
        let path = s.song.url.clone();
        let t = s.song.title().map(ToString::to_string);
        let a = s.song.artists().first().map(ToString::to_string);
        (path, t, a)
    } else {
        (String::new(), None, None)
    };

    let (queue_json, album_id) = {
        let q = query.lock().await;
        let album_id = q.path_lookup.get(&file_path).cloned();
        let track_metas: Vec<Option<serde_json::Value>> = queue
            .iter()
            .map(|s| q.track_lookup.get(&s.song.url).cloned())
            .collect();
        drop(q);

        let q_json: serde_json::Value = queue
            .iter()
            .enumerate()
            .zip(track_metas)
            .map(|((idx, s), track_meta)| {
                track_meta.map_or_else(
                    || {
                        serde_json::json!({
                            "id": idx,
                            "file": s.song.url,
                            "title": s.song.title(),
                            "artist": s.song.artists().first(),
                            "album_id": serde_json::Value::Null,
                            "track_no": serde_json::Value::Null,
                            "disc_no": 1,
                            "duration": "",
                            "duration_ms": 0,
                        })
                    },
                    |meta| {
                        serde_json::json!({
                            "id": idx,
                            "file": s.song.url,
                            "title": s.song.title(),
                            "artist": s.song.artists().first(),
                            "album_id": meta.get("albumId"),
                            "track_no": meta.get("trackNo"),
                            "disc_no": meta.get("discNo"),
                            "duration": meta.get("duration"),
                            "duration_ms": meta.get("durationMs"),
                        })
                    },
                )
            })
            .collect();

        (q_json, album_id)
    };

    let state_str = match status.state {
        PlayState::Playing => "play",
        PlayState::Paused => "pause",
        PlayState::Stopped => "stop",
    };

    let payload = serde_json::json!({
        "type": "MPD_STATUS",
        "state": state_str,
        "file": file_path,
        "album_id": album_id,
        "elapsed": status.elapsed.map_or(0.0, |t| t.as_secs_f64()),
        "duration": status.duration.map_or(0.0, |t| t.as_secs_f64()),
        "title": title,
        "artist": artist,
        "queue": queue_json
    });

    let _ = tx.send(payload.to_string());
    Ok(())
}
