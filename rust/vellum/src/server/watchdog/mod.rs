use crate::server::state::AppState;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use serde_json::json;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;
use std::collections::{HashMap, HashSet};
use crate::server::api::system::get_interface_config_path;

struct ChangeFlags {
    config: bool,
    logic: bool,
    shelf: bool,
    interfaces_config: HashSet<String>,
    interfaces_asset: HashSet<String>,
}

pub fn start(config_path: &Path, state: Arc<AppState>) {
    let (tx, rx) = tokio::sync::mpsc::channel::<Vec<PathBuf>>(10);
    let watcher = setup_watcher(tx);
    let canon_config_path = config_path.canonicalize().unwrap_or_else(|_| config_path.to_path_buf());
    let config_dir = canon_config_path.parent().unwrap_or_else(|| Path::new(".")).to_path_buf();

    tokio::spawn(async move {
        run_loop(rx, watcher, canon_config_path, config_dir, state).await;
    });
}

fn setup_watcher(tx: tokio::sync::mpsc::Sender<Vec<PathBuf>>) -> RecommendedWatcher {
    RecommendedWatcher::new(
        move |res: notify::Result<Event>| {
            if let Ok(event) = res
                && (event.kind.is_modify() || event.kind.is_create() || event.kind.is_remove())
            {
                let _ = tx.blocking_send(event.paths);
            }
        },
        notify::Config::default(),
    )
    .expect("Failed to create config watchdog")
}

async fn run_loop(
    mut rx: tokio::sync::mpsc::Receiver<Vec<PathBuf>>,
    mut watcher: RecommendedWatcher,
    canon_config_path: PathBuf,
    config_dir: PathBuf,
    state: Arc<AppState>,
) {
    let _ = watcher.watch(&config_dir, RecursiveMode::Recursive);

    let mut watched_shelves = get_initial_shelves(&state).await;
    for p in &watched_shelves {
        if p.exists() && !p.starts_with(&config_dir) {
            let _ = watcher.watch(p, RecursiveMode::NonRecursive);
        }
    }

    let mut watched_interfaces = get_initial_interfaces(&state).await;
    for p in watched_interfaces.values() {
        if p.exists() && !p.starts_with(&config_dir) {
            let _ = watcher.watch(p, RecursiveMode::Recursive);
        }
    }

    let mut watched_interface_configs = get_initial_interface_configs(&state).await;
    for p in watched_interface_configs.values() {
        if p.exists() && !p.starts_with(&config_dir) {
            let _ = watcher.watch(p, RecursiveMode::NonRecursive);
        }
    }

    while let Some(mut paths) = rx.recv().await {
        tokio::time::sleep(Duration::from_millis(100)).await;
        while let Ok(more_paths) = rx.try_recv() {
            paths.extend(more_paths);
        }

        let flags = classify_events(&paths, &canon_config_path, &watched_shelves, &watched_interfaces, &watched_interface_configs);
        process_events(flags, &state, &config_dir, &mut watcher, &mut watched_shelves, &mut watched_interfaces, &mut watched_interface_configs).await;
    }
}

async fn get_initial_shelves(state: &Arc<AppState>) -> Vec<PathBuf> {
    let guard = state.config.read().await;
    guard.resolved_shelf_files.clone()
}

async fn get_initial_interfaces(state: &Arc<AppState>) -> HashMap<String, PathBuf> {
    let guard = state.config.read().await;
    guard.interfaces.iter().map(|(name, cfg)| {
        let path = cfg.directory.as_ref().map_or_else(
            || {
                let p = libvellum::utils::expand_path(&format!("~/.local/share/vellum/interfaces/{name}"));
                p.canonicalize().unwrap_or(p)
            },
            |dir| {
                let p = libvellum::utils::expand_path(dir);
                let p = if p.is_absolute() { p } else { guard.config_dir.join(p) };
                p.canonicalize().unwrap_or(p)
            },
        );
        (name.clone(), path)
    }).collect()
}

async fn get_initial_interface_configs(state: &Arc<AppState>) -> HashMap<String, PathBuf> {
    let guard = state.config.read().await;
    guard.interfaces.iter().map(|(name, cfg)| {
        let config_path = get_interface_config_path(name, Some(cfg), &guard.config_dir);
        let config_path = config_path.canonicalize().unwrap_or(config_path);
        (name.clone(), config_path)
    }).collect()
}

fn classify_events(
    paths: &[PathBuf],
    canon_config_path: &Path,
    watched_shelves: &[PathBuf],
    watched_interfaces: &HashMap<String, PathBuf>,
    watched_interface_configs: &HashMap<String, PathBuf>,
) -> ChangeFlags {
    let mut flags = ChangeFlags {
        config: false,
        logic: false,
        shelf: false,
        interfaces_config: HashSet::new(),
        interfaces_asset: HashSet::new(),
    };

    for p in paths {
        let p_canon = p.canonicalize().unwrap_or_else(|_| p.clone());
        let path_str = p_canon.to_string_lossy();

        if path_str.contains("node_modules") || path_str.contains(".svelte-kit") || path_str.contains(".git") {
            continue;
        }

        if p_canon == *canon_config_path {
            flags.config = true;
        }

        if watched_shelves.contains(&p_canon) {
            flags.shelf = true;
        }

        if let Some(name) = p_canon.file_name().and_then(|n| n.to_str())
            && name == "logic.toml"
        {
            flags.logic = true;
        }

        for (name, cfg_path) in watched_interface_configs {
            if p_canon == *cfg_path {
                flags.interfaces_config.insert(name.clone());
            }
        }

        for (name, dir) in watched_interfaces {
            if p_canon.starts_with(dir) {
                if p_canon.file_name().is_some_and(|n| n == "config.toml") {
                    flags.interfaces_config.insert(name.clone());
                } else if !watched_interface_configs.values().any(|c| *c == p_canon) {
                    flags.interfaces_asset.insert(name.clone());
                }
            }
        }
    }
    flags
}

async fn process_events(
    flags: ChangeFlags,
    state: &Arc<AppState>,
    config_dir: &Path,
    watcher: &mut RecommendedWatcher,
    watched_shelves: &mut Vec<PathBuf>,
    watched_interfaces: &mut HashMap<String, PathBuf>,
    watched_interface_configs: &mut HashMap<String, PathBuf>,
) {
    if flags.logic {
        *watched_shelves = handle_logic_change(state, config_dir, watcher, watched_shelves).await;
    }

    if flags.shelf && !flags.logic {
        log::info!("Filesystem change: reloading shelf files...");
        {
            let mut query = state.query.lock().await;
            if let Err(e) = query.build_cache() {
                log::error!("Failed to rebuild query cache: {e}");
            }
        }
        let _ = state.tx.send(json!({ "type": "LOGIC_UPDATE" }).to_string());
    }

    for intf_name in flags.interfaces_config {
        log::info!("Interface '{intf_name}' config changed.");
        if let Some(cfg_path) = watched_interface_configs.get(&intf_name)
            && let Ok(content) = tokio::fs::read_to_string(cfg_path).await
            && let Ok(toml_val) = toml::from_str::<toml::Value>(&content)
        {
            let json_val = libvellum::types::toml_to_json(toml_val);
            let _ = state.tx.send(json!({
                "type": "INTERFACE_CONFIG_UPDATE",
                "name": intf_name,
                "config": json_val
            }).to_string());
        }
    }

    for intf_name in flags.interfaces_asset {
        log::info!("Interface '{intf_name}' asset changed.");
        let _ = state.tx.send(json!({
            "type": "INTERFACE_ASSET_UPDATE",
            "name": intf_name
        }).to_string());
    }

    if flags.config {
        let (updated_ints, updated_cfgs) = handle_config_change(state, config_dir, watcher, watched_interfaces, watched_interface_configs).await;
        *watched_interfaces = updated_ints;
        *watched_interface_configs = updated_cfgs;
    }
}

async fn handle_logic_change(
    state: &Arc<AppState>,
    config_dir: &Path,
    watcher: &mut RecommendedWatcher,
    current_shelves: &[PathBuf]
) -> Vec<PathBuf> {
    log::info!("Filesystem change: reloading logic.toml...");
    let logic_path = config_dir.join("logic.toml");
    let resolved = if logic_path.exists() { logic_path.canonicalize().ok() } else { None };

    let mut new_shelves = Vec::new();
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
                    new_shelves.push(expanded.canonicalize().unwrap_or(expanded));
                }
            }
        }
    }

    for p in current_shelves {
        if !new_shelves.contains(p) && !p.starts_with(config_dir) {
            let _ = watcher.unwatch(p);
        }
    }
    for p in &new_shelves {
        if !current_shelves.contains(p) && p.exists() && !p.starts_with(config_dir) {
            let _ = watcher.watch(p, RecursiveMode::NonRecursive);
        }
    }

    {
        let mut guard = state.config.write().await;
        guard.resolved_shelf_files.clone_from(&new_shelves);
    }

    let _ = state.tx.send(json!({ "type": "LOGIC_UPDATE" }).to_string());
    new_shelves
}

async fn handle_config_change(
    state: &Arc<AppState>,
    config_dir: &Path,
    watcher: &mut RecommendedWatcher,
    current_interfaces: &HashMap<String, PathBuf>,
    current_configs: &HashMap<String, PathBuf>,
) -> (HashMap<String, PathBuf>, HashMap<String, PathBuf>) {
    log::info!("Filesystem change: reloading config...");

    match libvellum::lua::ResolvedConfig::load() {
        Ok(new_config) => {
            let covers = new_config.covers.clone();
            let new_interfaces = new_config.app.interfaces.clone();

            let mut updated_interfaces = HashMap::new();
            for (name, cfg) in &new_interfaces {
                let dir = cfg.directory.as_ref().map_or_else(
                    || libvellum::utils::expand_path(&format!("~/.local/share/vellum/interfaces/{name}")),
                    |d| {
                        let p = libvellum::utils::expand_path(d);
                        if p.is_absolute() { p } else { config_dir.join(p) }
                    },
                );
                let new_p = dir.canonicalize().unwrap_or(dir);

                if new_p.exists() && !new_p.starts_with(config_dir) && !current_interfaces.values().any(|p| *p == new_p) {
                    let _ = watcher.watch(&new_p, RecursiveMode::Recursive);
                }
                updated_interfaces.insert(name.clone(), new_p);
            }

            for old_p in current_interfaces.values() {
                if !updated_interfaces.values().any(|p| p == old_p) && !old_p.starts_with(config_dir) {
                    let _ = watcher.unwatch(old_p);
                }
            }

            {
                let mut config_guard = state.config.write().await;
                config_guard.covers.clone_from(&covers);
                config_guard.interfaces.clone_from(&new_interfaces);
            }

            let mut updated_configs = HashMap::new();
            for (name, cfg) in &new_interfaces {
                let cfg_path = get_interface_config_path(name, Some(cfg), config_dir);
                let new_cp = cfg_path.canonicalize().unwrap_or(cfg_path);

                if new_cp.exists() && !new_cp.starts_with(config_dir) && !current_configs.values().any(|p| *p == new_cp) {
                    let _ = watcher.watch(&new_cp, RecursiveMode::NonRecursive);
                }
                updated_configs.insert(name.clone(), new_cp);
            }

            for old_cp in current_configs.values() {
                if !updated_configs.values().any(|p| p == old_cp) && !old_cp.starts_with(config_dir) {
                    let _ = watcher.unwatch(old_cp);
                }
            }

            let _ = state.tx.send(json!({
                "type": "CONFIG_UPDATE",
                "config": {
                    "covers": covers
                }
            }).to_string());

            (updated_interfaces, updated_configs)
        }
        Err(e) => {
            log::error!("Failed to reload config: {e:?}");
            (current_interfaces.clone(), current_configs.clone())
        }
    }
}
