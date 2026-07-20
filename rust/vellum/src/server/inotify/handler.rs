use crate::server::state::AppState;
use crate::server::inotify::classifier::ChangeFlags;
use serde_json::json;
use std::sync::Arc;

pub async fn process_events(
    flags: ChangeFlags,
    state: &Arc<AppState>,
) {
    if flags.logic {
        handle_logic_change(state).await;
    }

    if flags.shelf && !flags.logic {
        log::info!("Filesystem change: reloading shelf files...");
        {
            let mut query = state.query.write().await;
            if let Err(e) = query.build_cache() {
                log::error!("Failed to rebuild query cache: {e}");
            }
        }
        let _ = state.tx.send(json!({ "type": "LOGIC_UPDATE" }).to_string());
    }

    for intf_name in flags.interfaces_asset {
        log::info!("Interface '{intf_name}' asset changed.");
        let _ = state.tx.send(json!({
            "type": "INTERFACE_ASSET_UPDATE",
            "name": intf_name
        }).to_string());
    }

    if flags.config {
        handle_config_change(state).await;
    }
}

async fn handle_logic_change(state: &Arc<AppState>) {
    log::info!("Filesystem change: reloading logic.toml...");
    let logic_path = {
        let guard = state.config.read().await;
        guard.config_dir.join("logic.toml")
    };
    
    let resolved = if logic_path.exists() { logic_path.canonicalize().ok() } else { None };

    let mut new_shelves = Vec::new();
    {
        let mut guard = state.config.write().await;
        guard.resolved_logic_path.clone_from(&resolved);
    }

    if let Some(ref lp) = resolved {
        let mut query = state.query.write().await;
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

    {
        let mut guard = state.config.write().await;
        guard.resolved_shelf_files = new_shelves;
    }

    let _ = state.tx.send(json!({ "type": "LOGIC_UPDATE" }).to_string());
}

async fn handle_config_change(state: &Arc<AppState>) {
    log::info!("Filesystem change: reloading config...");

    match libvellum::lua::ResolvedConfig::load() {
        Ok(new_config) => {
            let covers = new_config.covers.clone();
            let new_interfaces = new_config.interfaces.clone();
            let dependencies = new_config.dependencies.clone();

            {
                let mut config_guard = state.config.write().await;
                config_guard.covers.clone_from(&covers);
                config_guard.interfaces.clone_from(&new_interfaces);
                config_guard.resolved_dependencies.clone_from(&dependencies);
            }

            let _ = state.tx.send(json!({
                "type": "CONFIG_UPDATE",
                "config": {
                    "covers": covers
                }
            }).to_string());

            for (name, cfg) in &new_interfaces {
                let _ = state.tx.send(json!({
                    "type": "INTERFACE_CONFIG_UPDATE",
                    "name": name,
                    "config": cfg.config
                }).to_string());
            }
        }
        Err(e) => {
            log::error!("Failed to reload config: {e:?}");
        }
    }
}
