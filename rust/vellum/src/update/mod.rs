use anyhow::{Context, Result};
use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use tokio::sync::mpsc;

use crate::compile;
use libvellum::config::AppConfig;
use libvellum::error::VellumError;
use libvellum::utils::expand_path;
use libvellum::sentinel::{TrustState, verify_trust};

#[derive(Serialize, Deserialize, Clone, Debug)]
struct AlbumCacheEntry {
    pub mtime_sum: u64,
}

#[derive(Serialize, Deserialize)]
struct CurrentState {
    pub hash: String,
}

struct NotificationTaskArgs {
    notify_rx: mpsc::Receiver<compile::engine::stream::AlbumUpdateSignal>,
    cache_for_task: Arc<Mutex<HashMap<String, AlbumCacheEntry>>>,
    exts_for_task: Vec<String>,
    manifests_for_task: Option<Vec<String>>,
    lib_root_for_task: Arc<PathBuf>,
    missing_paths: Vec<PathBuf>,
    start_time: std::time::Instant,
    verbose: bool,
    silent: bool,
}

pub async fn run(
    target_path: Option<PathBuf>,
    force: bool,
    jobs: Option<usize>,
    verbose: bool,
    silent: bool,
) -> Result<()> {
    let (config, _, _): (AppConfig, toml::Value, PathBuf) = AppConfig::load().context("Failed to load config")?;
    let library_root = expand_path(&config.storage.library_root)
        .canonicalize()
        .context("Invalid library_root")?;

    let is_full_library = target_path.is_none() || target_path.as_deref() == Some(library_root.as_path());

    if force && is_full_library {
        let client = reqwest::Client::new();
        let _ = client
            .post("http://127.0.0.1:8000/api/internal/notify_force_update")
            .timeout(std::time::Duration::from_secs(2))
            .send()
            .await;
    }

    let exts = config
        .manifest
        .as_ref()
        .and_then(|m| m.supported_extensions.clone())
        .unwrap_or_else(|| vec![".flac".to_string()]);

    let manifests = config.compiler.as_ref().and_then(|c| c.manifests.clone());

    let lib_hash = calculate_hash(&library_root.to_string_lossy());
    let base_cache_dir = expand_path(&config.storage.cache).join("libraries_cache");
    fs::create_dir_all(&base_cache_dir)?;

    validate_library_root(&base_cache_dir, &lib_hash).await?;

    let cache_file = base_cache_dir.join(format!("{lib_hash}.json"));
    let mut cache = load_cache(&cache_file);

    let scan_root = target_path.unwrap_or_else(|| library_root.clone());
    let scan_depth = config
        .compiler
        .as_ref()
        .and_then(|c| c.scan_depth)
        .unwrap_or(4);
    
    let all_albums = libvellum::scanner::find_target_albums(&scan_root, scan_depth)?;
    let missing_paths = find_missing_paths(&all_albums, &scan_root, &cache);

    if !silent {
        log::info!("Verifying {} albums...", all_albums.len());
    }

    let results = verify_albums_parallel(all_albums, &cache, force, jobs, &exts, manifests.as_ref())?;

    let mut work_queue = Vec::new();

    for (path, mtime, is_dirty) in results {
        if is_dirty {
            work_queue.push(path);
        } else {
            cache.insert(
                path.to_string_lossy().to_string(),
                AlbumCacheEntry { mtime_sum: mtime },
            );
        }
    }

    if work_queue.is_empty() && missing_paths.is_empty() {
        if !silent {
            log::info!("Library is up to date.");
        }
        save_cache(&cache, &cache_file)?;
        return Ok(());
    }

    let dirty_count = work_queue.len();
    let missing_count = missing_paths.len();
    let start_time = std::time::Instant::now();

    let (notify_tx, notify_rx) = mpsc::channel::<compile::engine::stream::AlbumUpdateSignal>(100);
    let cache_arc = Arc::new(Mutex::new(cache));

    let task_args = NotificationTaskArgs {
        notify_rx,
        cache_for_task: Arc::clone(&cache_arc),
        exts_for_task: exts.clone(),
        manifests_for_task: manifests.clone(),
        lib_root_for_task: Arc::new(library_root.clone()),
        missing_paths,
        start_time,
        verbose,
        silent,
    };
    let notification_task = start_notification_task(task_args);

    if !work_queue.is_empty() {
        let compile_options = compile::CompileOptions {
            target_path: scan_root,
            flags: vec!["default".to_string()],
            specific_albums: Some(work_queue),
            jobs,
            notify_tx: Some(notify_tx.clone()),
            compile_flags: compile::CompileFlags {
                mode: compile::CompileMode::Standard,
                target: compile::ExportTarget::File,
                pretty: false,
            },
        };
        compile::run(compile_options).await?;
    }

    drop(notify_tx);
    let _ = notification_task.await;

    let elapsed = start_time.elapsed().as_millis();
    if !silent {
        log::info!(
            "Update complete: {dirty_count} updated, {missing_count} removed. Finished in {elapsed}ms."
        );
    }

    let final_cache = Arc::try_unwrap(cache_arc).unwrap().into_inner();
    save_cache(&final_cache, &cache_file)?;

    Ok(())
}

fn find_missing_paths(all_albums: &[PathBuf], scan_root: &Path, cache: &HashMap<String, AlbumCacheEntry>) -> Vec<PathBuf> {
    let mut missing_paths = Vec::new();
    let album_set: HashSet<PathBuf> = all_albums.iter().cloned().collect();
    let scan_root_canon = scan_root.canonicalize().unwrap_or_else(|_| scan_root.to_path_buf());
    
    for cached_path_str in cache.keys() {
        let cached_path = PathBuf::from(cached_path_str);
        if cached_path.starts_with(&scan_root_canon) && !album_set.contains(&cached_path) {
            missing_paths.push(cached_path);
        }
    }
    missing_paths
}

fn start_notification_task(mut args: NotificationTaskArgs) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut updated_paths = Vec::new();
        while let Some(signal) = args.notify_rx.recv().await {
            if args.verbose && !args.silent {
                log::info!("Updated: {} - {}", signal.artist, signal.album);
            }
            updated_paths.push(signal.path);
        }

        let mut paths_for_server = Vec::new();

        {
            let mut c = args.cache_for_task.lock().await;
            for album_root in &updated_paths {
                let album_path_str = album_root.to_string_lossy().to_string();
                let metadata_path = album_root.join("metadata.toml");
                let mtime_sum = get_mtime_sum(album_root, &metadata_path, &args.exts_for_task, args.manifests_for_task.as_ref());
                c.insert(album_path_str.clone(), AlbumCacheEntry { mtime_sum });
                paths_for_server.push(album_path_str);
            }

            for missing in args.missing_paths {
                let p_str = missing.to_string_lossy().to_string();
                
                if args.verbose && !args.silent {
                    let display_path = missing.strip_prefix(&*args.lib_root_for_task).unwrap_or(&missing);
                    log::info!("Removed: {}", display_path.display());
                }

                c.remove(&p_str);
                paths_for_server.push(p_str);
            }
            drop(c);
        }

        if paths_for_server.is_empty() {
            return;
        }

        let client = reqwest::Client::new();
        let elapsed_ms = args.start_time.elapsed().as_millis();
        let url = format!("http://127.0.0.1:8000/api/internal/batch_reload?time={elapsed_ms}");

        let _ = client
            .post(&url)
            .json(&paths_for_server)
            .timeout(std::time::Duration::from_secs(30))
            .send()
            .await;
    })
}

async fn validate_library_root(cache_dir: &Path, current_hash: &str) -> Result<()> {
    let current_json_path = cache_dir.join("current.json");
    let mut needs_reset = false;

    if current_json_path.exists() {
        let content = fs::read_to_string(&current_json_path).unwrap_or_default();
        if let Ok(state) = serde_json::from_str::<CurrentState>(&content) {
            if state.hash != current_hash {
                needs_reset = true;
            }
        } else {
            needs_reset = true;
        }
    } else {
        needs_reset = true;
    }

    if needs_reset {
        log::info!("Library root changed. Triggering server reset.");
        let _ = fs::write(
            &current_json_path,
            serde_json::to_string(&CurrentState {
                hash: current_hash.to_string(),
            })?,
        );
        let _ = trigger_server_reset().await;
    }
    Ok(())
}

fn verify_albums_parallel(
    albums: Vec<PathBuf>,
    cache: &HashMap<String, AlbumCacheEntry>,
    force: bool,
    jobs: Option<usize>,
    exts: &[String],
    manifests: Option<&Vec<String>>,
) -> Result<Vec<(PathBuf, u64, bool)>> {
    let default_parallelism = std::thread::available_parallelism()
        .map(std::num::NonZero::get)
        .unwrap_or(1);
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(jobs.unwrap_or(default_parallelism))
        .build()?;

    Ok(pool.install(|| {
        albums
            .into_par_iter()
            .map(|album_root| {
                let album_path_str = album_root.to_string_lossy().to_string();
                let metadata_path = album_root.join("metadata.toml");
                let mtime_sum = get_mtime_sum(&album_root, &metadata_path, exts, manifests);

                if force {
                    return (album_root, mtime_sum, true);
                }

                if let Some(entry) = cache.get(&album_path_str)
                    && entry.mtime_sum == mtime_sum && mtime_sum != 0 {
                         return (album_root, mtime_sum, false);
                    }

                match verify_trust(&album_root, manifests) {
                    Ok(TrustState::Valid) => (album_root, mtime_sum, false),
                    Ok(_) => (album_root, mtime_sum, true),
                    Err(e) => {
                        match e {
                            VellumError::ManifestIoError(_) | VellumError::JsonError(_) => {
                                log::debug!("Cache Read Error for {}: {}. Forcing rebuild.", album_root.display(), e);
                            }
                            _ => {}
                        }
                        (album_root, mtime_sum, true)
                    }
                }
            })
            .collect()
    }))
}

fn calculate_hash(data: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn get_mtime_sum(dir: &Path, meta: &Path, exts: &[String], manifests: Option<&Vec<String>>) -> u64 {
    let d_mtime = fs::metadata(dir)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
        .unwrap_or(0);

    let mut m_mtime = fs::metadata(meta)
        .and_then(|m| m.modified())
        .map(|t| {
            t.duration_since(SystemTime::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs()
        })
        .unwrap_or(0);

    if let Some(names) = manifests {
        for name in names {
            let p = dir.join(name);
            if p.exists() {
                m_mtime += fs::metadata(&p)
                    .and_then(|m| m.modified())
                    .map(|t| {
                        t.duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs()
                    })
                    .unwrap_or(0);
            }
        }
    }

    let mut c_mtime = 0;
    let cover_candidates =["cover.jpg", "cover.png", "folder.jpg", "front.jpg"];

    for c in cover_candidates {
        let cp = dir.join(c);
        if cp.exists() {
            c_mtime = fs::metadata(cp)
                .and_then(|m| m.modified())
                .map(|t| {
                    t.duration_since(SystemTime::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs()
                })
                .unwrap_or(0);
            break;
        }
    }

    let mut t_mtime = 0;
    for entry in walkdir::WalkDir::new(dir)
        .max_depth(3)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
    {
        let p = entry.path();
        if p.is_file()
            && let Some(ext) = p.extension().and_then(|e| e.to_str()) {
                let ext_lower = format!(".{}", ext.to_lowercase());
                if exts.contains(&ext_lower) {
                    t_mtime += entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .map_or(0, |t| {
                            t.duration_since(SystemTime::UNIX_EPOCH)
                                .unwrap_or_default()
                                .as_secs()
                        });
                }
            }
    }

    d_mtime + m_mtime + c_mtime + t_mtime
}

fn load_cache(path: &Path) -> HashMap<String, AlbumCacheEntry> {
    if let Ok(content) = fs::read_to_string(path)
        && let Ok(cache) = serde_json::from_str::<HashMap<String, AlbumCacheEntry>>(&content) {
             return cache;
        }
    HashMap::new()
}

fn save_cache(cache: &HashMap<String, AlbumCacheEntry>, path: &Path) -> Result<()> {
    let content = serde_json::to_string_pretty(cache)?;
    fs::write(path, content)?;
    Ok(())
}

async fn trigger_server_reset() -> Result<()> {
    let client = reqwest::Client::new();
    let _ = client
        .post("http://127.0.0.1:8000/api/internal/reset")
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .await;
    Ok(())
}
