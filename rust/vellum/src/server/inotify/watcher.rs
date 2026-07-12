use crate::server::state::AppState;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use crate::server::inotify::classifier;
use crate::server::inotify::handler;

pub fn setup_watcher(tx: tokio::sync::mpsc::Sender<Vec<PathBuf>>) -> RecommendedWatcher {
    RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove())
            {
                let _ = tx.blocking_send(event.paths);
            }
        },
        notify::Config::default(),
    ).expect("Failed to create inotify watcher")
}

pub async fn run_loop(
    mut rx: tokio::sync::mpsc::Receiver<Vec<PathBuf>>,
    watcher: &mut RecommendedWatcher,
    state: Arc<AppState>,
) {
    let config_dir = {
        let guard = state.config.read().await;
        guard.config_dir.clone()
    };

    let mut watched_dirs = std::collections::HashSet::new();

    sync_watches(watcher, &mut watched_dirs, &state, &config_dir).await;

    while let Some(mut paths) = rx.recv().await {
        tokio::time::sleep(Duration::from_millis(100)).await;
        while let Ok(more_paths) = rx.try_recv() {
            paths.extend(more_paths);
        }

        let flags = classifier::classify_events(&paths, &state).await;
        handler::process_events(flags, &state).await;

        sync_watches(watcher, &mut watched_dirs, &state, &config_dir).await;
    }
}

async fn sync_watches(
    watcher: &mut RecommendedWatcher,
    watched_dirs: &mut std::collections::HashSet<PathBuf>,
    state: &Arc<AppState>,
    config_dir: &Path,
) {
    let guard = state.config.read().await;

    let mut needed_recursive = std::collections::HashSet::new();
    let mut needed_non_recursive = std::collections::HashSet::new();

    needed_recursive.insert(config_dir.to_path_buf());

    for cfg in guard.interfaces.values() {
        for asset_str in cfg.assets.values() {
            let p = libvellum::utils::expand_path(asset_str);
            let p = if p.is_absolute() { p } else { config_dir.join(p) };
            if let Ok(canon) = p.canonicalize() {
                if canon.is_dir() {
                    needed_recursive.insert(canon);
                } else if canon.is_file()
                    && let Some(parent) = canon.parent()
                {
                    needed_non_recursive.insert(parent.to_path_buf());
                }
            }
        }
    }

    for dep in &guard.resolved_dependencies {
        if let Some(parent) = dep.parent() {
            needed_non_recursive.insert(parent.to_path_buf());
        }
    }

    for shelf in &guard.resolved_shelf_files {
        if let Some(parent) = shelf.parent() {
            needed_non_recursive.insert(parent.to_path_buf());
        }
    }
    drop(guard);

    for dir in needed_recursive {
        if dir.exists() && watched_dirs.insert(dir.clone()) {
            let _ = watcher.watch(&dir, RecursiveMode::Recursive);
        }
    }

    for dir in needed_non_recursive {
        if dir.exists() && watched_dirs.insert(dir.clone()) {
            let _ = watcher.watch(&dir, RecursiveMode::NonRecursive);
        }
    }
}
