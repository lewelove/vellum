use crate::server::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;
use std::path::Path as StdPath;

pub fn get_interface_config_path(
    name: &str, 
    cfg: Option<&libvellum::config::InterfaceConfig>, 
    config_dir: &StdPath
) -> std::path::PathBuf {
    cfg.map_or_else(
        || {
            let default_path = libvellum::utils::expand_path(&format!("~/.local/share/vellum/interfaces/{name}/config.toml"));
            if default_path.is_absolute() {
                default_path
            } else {
                config_dir.join(default_path)
            }
        },
        |c| {
            c.config.as_ref().map_or_else(
                || {
                    let dir = c.directory.clone().unwrap_or_else(|| format!("~/.local/share/vellum/interfaces/{name}"));
                    let dir_path = libvellum::utils::expand_path(&dir);
                    let dir_abs = if dir_path.is_absolute() {
                        dir_path
                    } else {
                        config_dir.join(dir_path)
                    };
                    dir_abs.join("config.toml")
                },
                |cp| {
                    let p = libvellum::utils::expand_path(cp);
                    if p.is_absolute() {
                        p
                    } else {
                        config_dir.join(p)
                    }
                },
            )
        },
    )
}

pub async fn get_interface_config(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let (intf_cfg, config_dir) = {
        let guard = state.config.read().await;
        (guard.interfaces.get(&name).cloned(), guard.config_dir.clone())
    };
    let config_path = get_interface_config_path(&name, intf_cfg.as_ref(), &config_dir);
    if let Ok(content) = std::fs::read_to_string(&config_path)
        && let Ok(toml_val) = toml::from_str::<toml::Value>(&content)
    {
        let json_val = libvellum::types::toml_to_json(toml_val);
        return Json(json_val).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn serve_interface_asset(
    Path((name, asset_path)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let (intf_cfg, config_dir) = {
        let guard = state.config.read().await;
        (guard.interfaces.get(&name).cloned(), guard.config_dir.clone())
    };
    
    let dir = intf_cfg.as_ref()
        .and_then(|c| c.directory.as_ref())
        .map_or_else(
            || libvellum::utils::expand_path(&format!("~/.local/share/vellum/interfaces/{name}")),
            |c| {
                let p = libvellum::utils::expand_path(c);
                if p.is_absolute() {
                    p
                } else {
                    config_dir.join(p)
                }
            },
        );

    let full_path = dir.join(&asset_path);
    
    if !full_path.canonicalize().unwrap_or_default().starts_with(dir.canonicalize().unwrap_or_default()) {
        return StatusCode::FORBIDDEN.into_response();
    }

    if let Ok(mut file) = tokio::fs::File::open(&full_path).await {
        let mut buf = Vec::new();
        if tokio::io::AsyncReadExt::read_to_end(&mut file, &mut buf).await.is_ok() {
            let mime = match full_path.extension().and_then(|e| e.to_str()) {
                Some("css") => "text/css",
                Some("frag" | "glsl" | "vert") => "text/plain",
                Some("js") => "application/javascript",
                Some("json") => "application/json",
                Some("png") => "image/png",
                Some("jpg" | "jpeg") => "image/jpeg",
                Some("svg") => "image/svg+xml",
                Some("woff2") => "font/woff2",
                _ => "application/octet-stream",
            };
            return (
                [(axum::http::header::CONTENT_TYPE, mime)],
                buf
            ).into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn update_state(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    let content = {
        let mut ui = state.ui_state.write().await;
        if let Some(obj) = payload.as_object()
            && let Some(ui_obj) = ui.as_object_mut()
        {
            for (k, v) in obj {
                ui_obj.insert(k.clone(), v.clone());
            }
        }
        serde_json::to_string_pretty(&*ui).ok()
    };

    if let Some(data) = content {
        let state_file = {
            let guard = state.config.read().await;
            guard.state_root.join("state.json")
        };
        let _ = tokio::fs::write(state_file, data).await;
    }

    Json(json!({"status": "saved"})).into_response()
}

pub async fn notify_force_update() -> Response {
    log::info!("Force updating library...");
    Json(json!({"status": "ok"})).into_response()
}

pub async fn trigger_full_reset(State(state): State<Arc<AppState>>) -> Response {
    log::info!("Rebuilding library database...");
    let start_time = std::time::Instant::now();

    let album_count = {
        let library_root = state.config.read().await.library_root.clone();
        let scanner = crate::server::library::scanner::Library::new(library_root);
        let mut query = state.query.lock().await;
        scanner.scan(&mut query);
        query.dict.len()
    };

    let elapsed = start_time.elapsed().as_millis();
    log::info!("Updated {album_count} albums.");
    log::info!("Rebuilt Query Engine in {elapsed}ms.");

    let _ = state.tx.send(json!({"type": "LOGIC_UPDATE"}).to_string());
    Json(json!({"status": "ok"})).into_response()
}

pub async fn trigger_batch_reload(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
    Json(paths): Json<Vec<String>>,
) -> Response {
    let start_time = std::time::Instant::now();
    let compile_time = params.get("time").map_or("0", std::string::String::as_str);

    let library_root = state.config.read().await.library_root.clone();
    let mut processed_ids = Vec::new();
    let mut removed_ids = Vec::new();

    {
        let mut query = state.query.lock().await;
        let scanner = crate::server::library::scanner::Library::new(library_root);
        for path in &paths {
            let res = scanner.update_album(path, &mut query);
            match res {
                crate::server::library::scanner::UpdateResult::Updated(id) => processed_ids.push(id),
                crate::server::library::scanner::UpdateResult::Removed(id) => removed_ids.push(id),
            }
        }
        if !processed_ids.is_empty() || !removed_ids.is_empty() {
            let _ = query.build_cache();
        }
    }

    if !processed_ids.is_empty() || !removed_ids.is_empty() {
        let elapsed = start_time.elapsed().as_millis();
        log::info!("Updated {} albums, Removed {} albums in {}ms.", processed_ids.len(), removed_ids.len(), compile_time);
        log::info!("Rebuilt Query Engine in {elapsed}ms.");

        if processed_ids.len() == 1 && removed_ids.is_empty() {
            let (dict_entry, shelves) = {
                let query = state.query.lock().await;
                let entry = query.dict.get(&processed_ids[0]).cloned();
                let mut s = std::collections::HashMap::new();
                for key in query.manifest.shelves.keys() {
                    s.insert(key.clone(), query.request_shelf_view(key));
                }
                drop(query);
                (entry, s)
            };
            let _ = state.tx.send(json!({
                "type": "ALBUM_UPDATED",
                "id": processed_ids[0],
                "dictEntry": dict_entry.unwrap_or_else(|| json!({})),
                "shelves": shelves
            }).to_string());
        } else if removed_ids.len() == 1 && processed_ids.is_empty() {
            let shelves = {
                let query = state.query.lock().await;
                let mut s = std::collections::HashMap::new();
                for key in query.manifest.shelves.keys() {
                    s.insert(key.clone(), query.request_shelf_view(key));
                }
                drop(query);
                s
            };
            let _ = state.tx.send(json!({
                "type": "ALBUM_REMOVED",
                "id": removed_ids[0],
                "shelves": shelves
            }).to_string());
        } else {
            let _ = state.tx.send(json!({"type": "LOGIC_UPDATE"}).to_string());
        }
    }

    Json(processed_ids).into_response()
}

pub async fn trigger_reload(
    State(state): State<Arc<AppState>>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Response {
    let start_time = std::time::Instant::now();
    if let Some(path) = params.get("path") {
        let library_root = state.config.read().await.library_root.clone();

        let (update_res, dict_entry, shelves) = {
            let mut query = state.query.lock().await;
            let scanner = crate::server::library::scanner::Library::new(library_root);
            let res = scanner.update_album(path, &mut query);

            let entry = match &res {
                crate::server::library::scanner::UpdateResult::Updated(id) => {
                    let _ = query.build_cache();
                    query.dict.get(id).cloned()
                },
                crate::server::library::scanner::UpdateResult::Removed(_) => None,
            };
            
            let mut s = std::collections::HashMap::new();
            for key in query.manifest.shelves.keys() {
                s.insert(key.clone(), query.request_shelf_view(key));
            }
            drop(query);
            
            (res, entry, s)
        };

        let elapsed = start_time.elapsed().as_millis();
        log::info!("Processed 1 album.");
        log::info!("Rebuilt Query Engine in {elapsed}ms.");

        match update_res {
            crate::server::library::scanner::UpdateResult::Updated(id) => {
                let _ = state.tx.send(json!({
                    "type": "ALBUM_UPDATED",
                    "id": id,
                    "dictEntry": dict_entry.unwrap_or_else(|| json!({})),
                    "shelves": shelves
                }).to_string());
            }
            crate::server::library::scanner::UpdateResult::Removed(id) => {
                let _ = state.tx.send(json!({
                    "type": "ALBUM_REMOVED",
                    "id": id,
                    "shelves": shelves
                }).to_string());
            }
        }

        return Json(json!({"status": "ok"})).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn open_album_folder(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let path = {
        let config_guard = state.config.read().await;
        config_guard.library_root.join(id)
    };
    if path.exists() {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        return Json(json!({"status": "ok"})).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn open_lock_file(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let path = {
        let config_guard = state.config.read().await;
        config_guard.library_root.join(id).join("album.lock.json")
    };
    if path.exists() {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        return Json(json!({"status": "ok"})).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn open_manifest_file(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let path = {
        let config_guard = state.config.read().await;
        config_guard.library_root.join(id).join("metadata.toml")
    };
    if path.exists() {
        let _ = std::process::Command::new("xdg-open").arg(path).spawn();
        return Json(json!({"status": "ok"})).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn force_update_album(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let path = {
        let config_guard = state.config.read().await;
        config_guard.library_root.join(id)
    };
    if path.exists() {
        let _ = std::process::Command::new("vellum")
            .arg("update")
            .arg("--force")
            .arg("--silent")
            .arg(path)
            .spawn();
        return Json(json!({"status": "ok"})).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn run_query(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<serde_json::Value>,
) -> Response {
    let query = state.query.lock().await;
    let q_str = payload.get("query").and_then(|v| v.as_str()).unwrap_or("").trim();
    
    let sql = if q_str.is_empty() {
        "SELECT id FROM albums".to_string()
    } else {
        let upper_q = q_str.to_uppercase();
        if upper_q.starts_with("SELECT") {
            q_str.to_string()
        } else {
            let prefix = if upper_q.starts_with("WHERE")
                || upper_q.starts_with("ORDER")
                || upper_q.starts_with("LIMIT")
            {
                "SELECT id FROM albums "
            } else {
                "SELECT id FROM albums WHERE "
            };
            format!("{prefix}{q_str}")
        }
    };

    let expanded = crate::server::query::expand_shorthand(&sql);
    match query.query_ids(&expanded) {
        Ok(ids) => Json(ids).into_response(),
        Err(e) => (StatusCode::BAD_REQUEST, Json(json!({"error": e.to_string()}))).into_response(),
    }
}
