use anyhow::{Context, Result};
use libvellum::utils::expand_path;
use mpd_client::Client;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use tokio::net::TcpStream;

pub struct TargetFlags {
  pub playing: bool,
  pub id: Option<String>,
  pub query: Option<String>,
  pub directory: Option<String>,
  pub recursive: Option<String>,
  pub library: bool,
}

pub fn load_env_vars_from_path(env_path: Option<&str>) -> std::collections::HashMap<String, String> {
  let mut env_vars = std::collections::HashMap::new();
  if let Some(path_str) = env_path {
      let expanded = expand_path(path_str);
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

async fn resolve_target_ids(
  library_root: &Path,
  target: &TargetFlags,
) -> Result<Vec<String>> {
  let default_dir = if !target.playing
      && target.id.is_none()
      && target.query.is_none()
      && target.directory.is_none()
      && target.recursive.is_none()
      && !target.library
  {
      Some(".".to_string())
  } else {
      target.directory.clone()
  };

  if let Some(query_str) = &target.query {
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

      res.json().await.context("Invalid response from server")
  } else if let Some(id) = &target.id {
      Ok(vec![id.clone()])
  } else if target.playing {
      let playing_path = get_playing_album(&library_root.to_string_lossy()).await?;
      let id = playing_path
          .strip_prefix(library_root)
          .map_or_else(
              |_| playing_path.to_string_lossy().to_string(),
              |p| p.to_string_lossy().to_string(),
          );
      Ok(vec![id])
  } else if target.library {
      let mut ids = Vec::new();
      for entry in walkdir::WalkDir::new(library_root).follow_links(true).into_iter().filter_map(Result::ok) {
          if entry.file_name() == "album.lock.json"
              && let Ok(content) = std::fs::read_to_string(entry.path())
              && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
              && let Some(id) = json.pointer("/album/id").and_then(|v| v.as_str())
          {
              ids.push(id.to_string());
          }
      }
      Ok(ids)
  } else if let Some(dir) = &target.recursive {
      let mut ids = Vec::new();
      for entry in walkdir::WalkDir::new(expand_path(dir)).follow_links(true).into_iter().filter_map(Result::ok) {
          if entry.file_name() == "album.lock.json"
              && let Ok(content) = std::fs::read_to_string(entry.path())
              && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
              && let Some(id) = json.pointer("/album/id").and_then(|v| v.as_str())
          {
              ids.push(id.to_string());
          }
      }
      Ok(ids)
  } else if let Some(dir) = &default_dir {
      let p = expand_path(dir).join("album.lock.json");
      if p.exists() {
          if let Ok(content) = std::fs::read_to_string(&p)
              && let Ok(json) = serde_json::from_str::<serde_json::Value>(&content)
              && let Some(id) = json.pointer("/album/id").and_then(|v| v.as_str())
          {
              Ok(vec![id.to_string()])
          } else {
              Ok(vec![])
          }
      } else {
          Ok(vec![])
      }
  } else {
      Ok(vec![])
  }
}

pub async fn execute(
  name: String,
  target: TargetFlags,
  debug: bool,
  trailing_args: Vec<String>,
) -> Result<()> {
  let name_key = name.replace('-', "_");
  let config = libvellum::lua::ResolvedConfig::load().context("Failed to load config")?;
  let config_dir = config.path.parent().unwrap_or_else(|| Path::new("."));

  let library_root = expand_path(&config.app.storage.library)
      .canonicalize()
      .unwrap_or_else(|_| expand_path(&config.app.storage.library));

  let target_ids = resolve_target_ids(&library_root, &target).await?;

  let mut lock_jsons = Vec::new();
  for target_id in &target_ids {
      let lock_file_path = library_root.join(target_id).join("album.lock.json");
      if let Ok(json_data) = std::fs::read_to_string(&lock_file_path)
          && let Ok(lock_json) = serde_json::from_str::<serde_json::Value>(&json_data)
      {
          lock_jsons.push(lock_json);
      }
  }

  let action_cfg_opt = config.actions.get(&name_key).cloned();

  let config_json = serde_json::to_value(&config.app)?;
  let combined_json = serde_json::json!({
      "albums": lock_jsons,
      "config": {
          "vellum": config_json,
          "action": action_cfg_opt.as_ref().map(|c| c.config.clone()).unwrap_or_default()
      },
      "options": trailing_args.join(" ")
  });

  if name == "intermediary" {
      let pretty_json = serde_json::to_string_pretty(&combined_json)?;
      println!("{pretty_json}");
      return Ok(());
  }

  let Some(action_cfg) = action_cfg_opt else {
      anyhow::bail!("Action '{name}' is not declared in configuration.");
  };

  let run_str = action_cfg.run.unwrap_or_else(|| format!("~/.local/share/vellum/actions/{name_key}.sh"));
  let expanded_action_path = expand_path(&run_str);

  let action_path = if expanded_action_path.is_absolute() {
      expanded_action_path
  } else {
      config_dir.join(expanded_action_path)
  };

  if !action_path.exists() {
      anyhow::bail!("Action path '{}' does not exist.", action_path.display());
  }

  let env_vars = load_env_vars_from_path(config.app.storage.environment.as_deref());

  if debug {
      log::debug!("Executing action '{name}' for {} album(s)", target_ids.len());
  }

  let cmd = if action_path.extension().is_some_and(|e| e == "py") {
      "python"
  } else if action_path.extension().is_some_and(|e| e == "sh") {
      "sh"
  } else {
      action_path.to_str().unwrap()
  };

  let mut command = tokio::process::Command::new(cmd);
  command.envs(&env_vars);
  if cmd == "python" || cmd == "sh" {
      command.arg(&action_path);
  }

  let mut child = command
      .stdin(Stdio::piped())
      .stdout(Stdio::inherit())
      .stderr(Stdio::inherit())
      .spawn()
      .context(format!("Failed to spawn action at {}", action_path.display()))?;

  if let Some(mut stdin) = child.stdin.take() {
      let payload = serde_json::to_string(&combined_json)?;
      tokio::spawn(async move {
          use tokio::io::AsyncWriteExt;
          let _ = stdin.write_all(payload.as_bytes()).await;
      });
  }

  tokio::select! {
      res = child.wait() => {
          let status = res.context("Failed to wait on action")?;
          if !status.success() {
              log::error!("Action failed with status: {status}");
          } else if debug {
              log::debug!("Action completed successfully.");
          }
      }
      _ = tokio::signal::ctrl_c() => {
          let _ = child.wait().await;
      }
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
