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

    let mut interface_dirs = std::collections::HashMap::new();
    for (name, cfg) in &guard.interfaces {
        let dir = cfg.directory.as_ref().map_or_else(
            || {
                let p = libvellum::utils::expand_path(&format!("~/.local/share/vellum/interfaces/{name}"));
                p.canonicalize().unwrap_or(p)
            },
            |d| {
                let p = libvellum::utils::expand_path(d);
                let p = if p.is_absolute() { p } else { config_dir.join(p) };
                p.canonicalize().unwrap_or(p)
            },
        );
        interface_dirs.insert(name.clone(), dir);
    }
    drop(guard);

    for p in paths {
        let p_canon = p.canonicalize().unwrap_or_else(|_| p.clone());
        let path_str = p_canon.to_string_lossy();

        if path_str.contains("node_modules") || path_str.contains(".svelte-kit") || path_str.contains(".git") {
            continue;
        }

        if deps.contains(&p_canon) {
            flags.config = true;
        }

        if shelves.contains(&p_canon) {
            flags.shelf = true;
        }

        if p_canon == canon_logic {
            flags.logic = true;
        }

        for (name, dir) in &interface_dirs {
            if p_canon.starts_with(dir) {
                flags.interfaces_asset.insert(name.clone());
            }
        }
    }

    flags
}
