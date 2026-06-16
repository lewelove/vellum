use anyhow::{Context, Result};
use libvellum::config::AppConfig;
use libvellum::utils::expand_path;
use mpd_client::Client;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tokio::net::TcpStream;

fn load_env_vars(config: &AppConfig) -> std::collections::HashMap<String, String> {
    let mut env_vars = std::collections::HashMap::new();
    if let Some(env_path) = &config.storage.environment {
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
    intermediary: bool,
) -> Result<()> {
    let (config, _, config_path) = AppConfig::load().context("Failed to load config")?;
    let config_dir = config_path.parent().unwrap_or_else(|| Path::new("."));

    if !playing && id_arg.is_none() {
        anyhow::bail!("At least one flag (--playing or --id) must be provided.");
    }

    let library_root = expand_path(&config.storage.library_root)
        .canonicalize()
        .unwrap_or_else(|_| expand_path(&config.storage.library_root));

    let target_id = if let Some(id) = id_arg {
        id
    } else if playing {
        let playing_path = get_playing_album(&config.storage.library_root).await?;
        playing_path
            .strip_prefix(&library_root)
            .map_or_else(
                |_| playing_path.to_string_lossy().to_string(),
                |p| p.to_string_lossy().to_string(),
            )
    } else {
        unreachable!();
    };

    let lock_file_path = library_root.join(&target_id).join("metadata.lock.json");
    let json_data = std::fs::read_to_string(&lock_file_path).context(format!(
        "Failed to read metadata.lock.json for album '{target_id}'"
    ))?;

    let lock_json: serde_json::Value = serde_json::from_str(&json_data)?;
    let config_json = serde_json::to_value(&config)?;
    let combined_json = serde_json::json!({
        "config": config_json,
        "album": lock_json
    });

    if intermediary {
        let pretty_json = serde_json::to_string_pretty(&combined_json)?;
        println!("{pretty_json}");
        return Ok(());
    }

    let script_rel_path = config
        .scripts
        .as_ref()
        .and_then(|s| s.get(&name))
        .context(format!("Script '{name}' not found in config.toml [scripts] section"))?;

    let script_path = if Path::new(script_rel_path).is_absolute() {
        PathBuf::from(script_rel_path)
    } else {
        config_dir.join(script_rel_path)
    };

    if !script_path.exists() {
        anyhow::bail!("Script path '{}' does not exist.", script_path.display());
    }

    let env_vars = load_env_vars(&config);

    log::info!("Executing script '{name}' for album '{target_id}'");

    let cmd = if script_path.extension().is_some_and(|e| e == "py") {
        "python"
    } else if script_path.extension().is_some_and(|e| e == "sh") {
        "sh"
    } else {
        script_path.to_str().unwrap()
    };

    let mut command = Command::new(cmd);
    command.envs(&env_vars);
    if cmd == "python" || cmd == "sh" {
        command.arg(&script_path);
    }

    let mut child = command
        .stdin(Stdio::piped())
        .spawn()
        .context(format!("Failed to spawn script at {}", script_path.display()))?;

    if let Some(mut stdin) = child.stdin.take() {
        let payload = serde_json::to_string(&combined_json)?;
        stdin
            .write_all(payload.as_bytes())
            .context("Failed to write to script stdin")?;
    }

    let status = child.wait().context("Failed to wait on script")?;

    if status.success() {
        log::info!("Script completed successfully.");
    } else {
        log::error!("Script failed with status: {status}");
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
