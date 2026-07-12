use crate::server::state::AppState;
use std::path::PathBuf;
use std::sync::Arc;
use std::collections::HashSet;

pub struct ChangeFlags {
    pub config: bool,
    pub logic: bool,
    pub shelf: bool,
    pub interfaces_asset: HashSet<String>,
}

pub async fn classify_events(
    paths: &[PathBuf],
    state: &Arc<AppState>,
) -> ChangeFlags {
    let mut flags = ChangeFlags {
        config: false,
        logic: false,
        shelf: false,
        interfaces_asset: HashSet::new(),
    };

    let guard = state.config.read().await;
    let config_dir = guard.config_dir.clone();
    let logic_path = config_dir.join("logic.toml");
    let canon_logic = logic_path.canonicalize().unwrap_or(logic_path);
    
    let shelves: Vec<PathBuf> = guard.resolved_shelf_files.clone();
    let deps: Vec<PathBuf> = guard.resolved_dependencies.clone();

    let mut interface_assets: Vec<(String, PathBuf, bool)> = Vec::new();
    for (name, cfg) in &guard.interfaces {
        for asset_str in cfg.assets.values() {
            let p = libvellum::utils::expand_path(asset_str);
            let p = if p.is_absolute() { p } else { config_dir.join(p) };
            if let Ok(canon) = p.canonicalize() {
                interface_assets.push((name.clone(), canon.clone(), canon.is_dir()));
            }
        }
    }
    drop(guard);

    for p in paths {
        let p_canon = p.canonicalize().unwrap_or_else(|_| p.clone());

        if deps.contains(&p_canon) {
            flags.config = true;
        }

        if shelves.contains(&p_canon) {
            flags.shelf = true;
        }

        if p_canon == canon_logic {
            flags.logic = true;
        }

        for (name, asset_path, is_dir) in &interface_assets {
            if *is_dir {
                if p_canon.starts_with(asset_path) {
                    flags.interfaces_asset.insert(name.clone());
                }
            } else if p_canon == *asset_path {
                flags.interfaces_asset.insert(name.clone());
            }
        }
    }

    flags
}
