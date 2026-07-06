use anyhow::{Context, Result};
use libvellum::utils::expand_path;
use mpd_client::Client;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::net::TcpStream;

fn load_env_vars(config: &libvellum::lua::ResolvedConfig) -> std::collections::HashMap<String, String> {
    let mut env_vars = std::collections::HashMap::new();
    if let Some(env_path) = &config.app.storage.environment {
        let expanded = expand_path(env_path);
        if let Ok(content) = std::fs::read_to_string(&expanded) {
            for line in content.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((k, v)) = line.split_once('=') {
                    env_vars.insert(
                        k.trim().to_string(),
                        v.trim().trim_matches(|c| c == '"' || c == '\'').to_string(),
                    );
                }
            }
        }
    }
    env_vars
}

pub async fn execute(
    name: String,
    playing: bool,
    id_arg: Option<String>,
    query_arg: Option<String>,
) -> Result<()> {
    let config = libvellum::lua::ResolvedConfig::load().context("Failed to load config")?;
    let config_dir = config.path.parent().unwrap_or_else(|| Path::new("."));

    if !playing && id_arg.is_none() && query_arg.is_none() {
        anyhow::bail!("At least one flag (--playing, --id, or --query) must be provided.");
    }

    let library_root = expand_path(&config.app.storage.library)
        .canonicalize()
        .unwrap_or_else(|_| expand_path(&config.app.storage.library));

    let target_ids = if let Some(query_str) = query_arg {
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:8000/api/internal/query")
            .json(&serde_json::json!({ 
                "query": query_str
            }))
            .send()
            .await
            .context("Failed to connect to the Vellum server. Is it running?")?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            anyhow::bail!("Server rejected query: {err_text}");
        }

        res.json().await.context("Invalid response from server")?
    } else if let Some(id) = id_arg {
        vec![id]
    } else if playing {
        let playing_path = get_playing_album(&config.app.storage.library).await?;
        let id = playing_path
            .strip_prefix(&library_root)
            .map_or_else(
                |_| playing_path.to_string_lossy().to_string(),
                |p| p.to_string_lossy().to_string(),
            );
        vec![id]
    } else {
        unreachable!();
    };

    let mut lock_jsons = Vec::new();
    for target_id in &target_ids {
        let lock_file_path = library_root.join(target_id).join("album.lock.json");
        if let Ok(json_data) = std::fs::read_to_string(&lock_file_path)
            && let Ok(lock_json) = serde_json::from_str::<serde_json::Value>(&json_data)
        {
            lock_jsons.push(lock_json);
        }
    }

    let config_json = serde_json::to_value(&config.app)?;
    let combined_json = serde_json::json!([lock_jsons, config_json]);

    if name == "intermediary" {
        let pretty_json = serde_json::to_string_pretty(&combined_json)?;
        println!("{pretty_json}");
        return Ok(());
    }

    let action_rel_path = config.app
        .actions
        .get(&name)
        .context(format!("Action '{name}' not found in config"))?;

    let expanded_action_path = expand_path(action_rel_path);

    let action_path = if expanded_action_path.is_absolute() {
        expanded_action_path
    } else {
        config_dir.join(expanded_action_path)
    };

    if !action_path.exists() {
        anyhow::bail!("Action path '{}' does not exist.", action_path.display());
    }

    let env_vars = load_env_vars(&config);

    log::info!("Executing action '{name}' for {} album(s)", target_ids.len());

    let cmd = if action_path.extension().is_some_and(|e| e == "py") {
        "python"
    } else if action_path.extension().is_some_and(|e| e == "sh") {
        "sh"
    } else {
        action_path.to_str().unwrap()
    };

    let mut command = Command::new(cmd);
    command.envs(&env_vars);
    if cmd == "python" || cmd == "sh" {
        command.arg(&action_path);
    }

    let mut child = command
        .stdin(Stdio::piped())
        .spawn()
        .context(format!("Failed to spawn action at {}", action_path.display()))?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = serde_json::to_string(&combined_json)?;
        stdin
            .write_all(payload.as_bytes())
            .context("Failed to write to action stdin")?;
    }

    let status = child.wait().context("Failed to wait on action")?;

    if status.success() {
        log::info!("Action completed successfully.");
    } else {
        log::error!("Action failed with status: {status}");
    }

    Ok(())
}

pub async fn get_playing_album(lib_root: &str) -> Result<PathBuf> {
    let host = std::env::var("MPD_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("MPD_PORT").unwrap_or_else(|_| "6600".to_string());
    let addr = format!("{host}:{port}");

    let stream = TcpStream::connect(&addr)
        .await
        .context("Failed to connect to MPD")?;
    let (client, _) = Client::connect(stream)
        .await
        .context("Failed to initialize MPD client")?;

    let current_song = client.command(mpd_client::commands::CurrentSong).await?;
    let song = current_song.context("No song is currently playing")?;

    let rel_path = song.song.url;
    let root = expand_path(lib_root);
    let full_path = root.join(rel_path);

    let album_dir = full_path
        .parent()
        .context("Invalid track path")?
        .to_path_buf();

    Ok(album_dir)
}
