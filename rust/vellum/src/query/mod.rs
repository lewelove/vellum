use anyhow::{Context, Result};
use libvellum::config::AppConfig;
use libvellum::utils::expand_path;

pub struct QueryFlags {
    pub playing: bool,
    pub lock: bool,
    pub id: bool,
    pub uid: bool,
    pub json: bool,
}

pub async fn run(query_str: Option<String>, flags: QueryFlags) -> Result<()> {
    let (config, _, _): (AppConfig, toml::Value, std::path::PathBuf) = AppConfig::load().context("Failed to load config")?;
    let lib_root = expand_path(&config.storage.library_root)
        .canonicalize()
        .unwrap_or_else(|_| expand_path(&config.storage.library_root));

    let mut target_ids = Vec::new();

    if query_str.is_some() {
        let client = reqwest::Client::new();
        let res = client
            .post("http://127.0.0.1:8000/api/internal/query")
            .json(&serde_json::json!({ 
                "query": query_str.unwrap_or_default()
            }))
            .send()
            .await
            .context("Failed to connect to the Vellum server. Is it running?")?;

        if !res.status().is_success() {
            let err_text = res.text().await.unwrap_or_default();
            anyhow::bail!("Server rejected query: {err_text}");
        }

        let ids: Vec<String> = res.json().await.context("Invalid response from server")?;
        target_ids = ids;
    } else if flags.playing {
        let playing_path = crate::x::get_playing_album(&config.storage.library_root).await?;
        let rel_path = playing_path.strip_prefix(&lib_root).map_or_else(|_| playing_path.to_string_lossy().to_string(), |p| p.to_string_lossy().to_string());
        target_ids.push(rel_path);
    } else {
        anyhow::bail!("No query provided. Use --playing or provide an SQL query.");
    }

    if flags.json {
        let mut albums = Vec::new();
        for id in &target_ids {
            let lock_file_path = lib_root.join(id).join("album.lock.json");
            if let Ok(content) = std::fs::read_to_string(&lock_file_path)
                && let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&content)
            {
                albums.push(json_val);
            }
        }
        println!("{}", serde_json::to_string_pretty(&albums)?);
    } else {
        for id in target_ids {
            if flags.id {
                println!("{id}");
            } else if flags.uid {
                let base_path = lib_root.join(&id);
                println!("{}", base_path.display());
            } else if flags.lock {
                let base_path = lib_root.join(&id);
                let final_path = base_path.join("album.lock.json");
                println!("{}", final_path.display());
            } else {
                let base_path = lib_root.join(&id);
                println!("{}", base_path.display());
            }
        }
    }

    Ok(())
}
