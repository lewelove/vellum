use libvellum::config::AppConfig;
use crate::server::state::AppState;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

struct ChangeFlags {
    config_changed: bool,
    css_changed: bool,
    logic_changed: bool,
    shelf_changed: bool,
}

pub fn start(config_path: &Path, state: Arc<AppState>) {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<Vec<PathBuf>>(10);
    
    let canon_config_path = config_path.canonicalize().unwrap_or_else(|_| config_path.to_path_buf());
    let config_dir = canon_config_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    tokio::spawn(async move {
        let tx_clone = tx.clone();
        let mut watcher = RecommendedWatcher::new(
            move |res: notify::Result<Event>| {
                if let Ok(event) = res
                    && (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove()) {
                        let _ = tx_clone.blocking_send(event.paths);
                    }
            },
            notify::Config::default(),
        )
        .expect("Failed to create config watcher");

        watcher
            .watch(&config_dir, RecursiveMode::Recursive)
            .expect("Failed to watch config directory");

        let mut current_watched_shader: Option<PathBuf> = {
            let guard = state.config.read().await;
            guard.resolved_shader_path.clone()
        };

        if let Some(ref p) = current_watched_shader
            && p.exists() && !p.starts_with(&config_dir) {
                let _ = watcher.watch(p, RecursiveMode::NonRecursive);
            }

        let mut current_watched_shelf_files: Vec<PathBuf> = {
            let guard = state.config.read().await;
            guard.resolved_shelf_files.clone()
        };

        for p in &current_watched_shelf_files {
            if p.exists() && !p.starts_with(&config_dir) {
                let _ = watcher.watch(p, RecursiveMode::NonRecursive);
            }
        }

        while let Some(mut paths) = rx.recv().await {
            tokio::time::sleep(Duration::from_millis(100)).await;
            while let Ok(more_paths) = rx.try_recv() {
                paths.extend(more_paths);
            }

            let flags = classify_events(&paths, &canon_config_path, current_watched_shader.as_ref(), &current_watched_shelf_files);

            if flags.css_changed {
                log::info!("Filesystem change: reloading custom CSS...");
                let mut guard = state.config.write().await;
                let css_path = config_dir.join("vellum.css");
                guard.resolved_css_path = if css_path.exists() { css_path.canonicalize().ok() } else { None };
                
                let payload = json!({ "type": "THEME_UPDATE" }).to_string();
                let _ = state.tx.send(payload);
            }

            if flags.logic_changed {
                current_watched_shelf_files = handle_logic_change(
                    &state, 
                    &config_dir, 
                    &mut watcher, 
                    &current_watched_shelf_files
                ).await;
            }

            if flags.shelf_changed && !flags.logic_changed {
                log::info!("Filesystem change: reloading shelf files...");
                {
                    let mut query = state.query.lock().await;
                    if let Err(e) = query.build_cache() {
                        log::error!("Failed to rebuild query cache: {e}");
                    }
                }
                let payload = json!({ "type": "LOGIC_UPDATE" }).to_string();
                let _ = state.tx.send(payload);
            }

            if flags.config_changed {
                current_watched_shader = handle_config_change(
                    &state, 
                    &config_dir, 
                    &mut watcher, 
                    current_watched_shader.as_ref()
                ).await;
            }
        }
    });
}

fn classify_events(
    paths: &[PathBuf],
    canon_config_path: &Path,
    current_watched_shader: Option<&PathBuf>,
    current_watched_shelf_files: &[PathBuf]
) -> ChangeFlags {
    let mut flags = ChangeFlags {
        config_changed: false,
        css_changed: false,
        logic_changed: false,
        shelf_changed: false,
    };

    for p in paths {
        let p = p.canonicalize().unwrap_or_else(|_| p.clone());

        if p == *canon_config_path {
            flags.config_changed = true;
        }

        if let Some(sp) = current_watched_shader
            && p == *sp {
                flags.config_changed = true;
            }

        if current_watched_shelf_files.contains(&p) {
            flags.shelf_changed = true;
        }

        if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
            match name {
                "vellum.css" => {
                    flags.css_changed = true;
                }
                "logic.toml" => {
                    flags.logic_changed = true;
                }
                _ => {}
            }
        }
    }
    flags
}

async fn handle_logic_change(
    state: &Arc<AppState>,
    config_dir: &Path,
    watcher: &mut RecommendedWatcher,
    current_watched_shelf_files: &[PathBuf]
) -> Vec<PathBuf> {
    log::info!("Filesystem change: reloading logic.toml...");
    let logic_path = config_dir.join("logic.toml");
    let resolved = if logic_path.exists() { logic_path.canonicalize().ok() } else { None };
    
    let mut new_shelf_files = Vec::new();

    {
        let mut guard = state.config.write().await;
        guard.resolved_logic_path.clone_from(&resolved);
    }

    if let Some(ref lp) = resolved {
        let mut query = state.query.lock().await;
        if let Err(e) = query.reload_manifest(lp) {
            log::error!("Failed to reload logic.toml: {e}");
        } else {
            for shelf in query.manifest.shelves.values() {
                if let Some(file) = &shelf.file {
                    let expanded = libvellum::utils::expand_path(file);
                    new_shelf_files.push(expanded.canonicalize().unwrap_or(expanded));
                }
            }
        }
    }

    for p in current_watched_shelf_files {
        if !new_shelf_files.contains(p) && !p.starts_with(config_dir) {
            let _ = watcher.unwatch(p);
        }
    }
    for p in &new_shelf_files {
        if !current_watched_shelf_files.contains(p) && p.exists() && !p.starts_with(config_dir) {
            let _ = watcher.watch(p, RecursiveMode::NonRecursive);
        }
    }
    
    {
        let mut guard = state.config.write().await;
        guard.resolved_shelf_files.clone_from(&new_shelf_files);
    }

    let payload = json!({ "type": "LOGIC_UPDATE" }).to_string();
    let _ = state.tx.send(payload);

    new_shelf_files
}

async fn handle_config_change(
    state: &Arc<AppState>,
    config_dir: &Path,
    watcher: &mut RecommendedWatcher,
    current_watched_shader: Option<&PathBuf>
) -> Option<PathBuf> {
    log::info!("Filesystem change: reloading config...");

    match AppConfig::load() {
        Ok((new_config, _, _)) => {
            let covers = new_config.compiler.as_ref().map(|c| c.covers.clone()).unwrap_or_default();
            let shader_cfg = new_config.theme.as_ref().and_then(|t| t.shader.clone());

            let next_shader_path = if let Some(s) = &shader_cfg
                && let Some(p) = &s.path {
                    let expanded = libvellum::utils::expand_path(p);
                    let absolute = if expanded.is_absolute() {
                        expanded
                    } else {
                        config_dir.join(expanded)
                    };
                    absolute.canonicalize().ok().or(Some(absolute))
                } else {
                    None
                };

            let mut updated_shader = current_watched_shader.cloned();

            if next_shader_path != updated_shader {
                if let Some(ref old_p) = updated_shader
                    && !old_p.starts_with(config_dir) {
                        let _ = watcher.unwatch(old_p);
                    }
                if let Some(ref new_p) = next_shader_path
                    && new_p.exists() && !new_p.starts_with(config_dir) {
                        let _ = watcher.watch(new_p, RecursiveMode::NonRecursive);
                    }
                updated_shader = next_shader_path.clone();
            }

            {
                let mut config_guard = state.config.write().await;
                config_guard.covers.clone_from(&covers);
                config_guard.shader.clone_from(&shader_cfg);
                config_guard.resolved_shader_path.clone_from(&next_shader_path);
            }

            let payload = json!({
                "type": "CONFIG_UPDATE",
                "config": {
                    "covers": covers,
                    "shader": shader_cfg
                }
            })
            .to_string();

            let _ = state.tx.send(payload);

            updated_shader
        }
        Err(e) => {
            log::error!("Failed to reload config: {e}");
            current_watched_shader.cloned()
        }
    }
}
