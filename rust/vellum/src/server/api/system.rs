use crate::server::state::AppState;
use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;
use std::sync::Arc;

pub async fn get_interface_config(
    Path(name): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let name = name.replace('-', "_");
    let intf_cfg = {
        let guard = state.config.read().await;
        guard.interfaces.get(&name).cloned()
    };
    if let Some(cfg) = intf_cfg {
        return Json(cfg.config).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn serve_interface_asset(
    Path((name, asset_path)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let name = name.replace('-', "_");
    let (intf_cfg, config_dir) = {
        let guard = state.config.read().await;
        (guard.interfaces.get(&name).cloned(), guard.config_dir.clone())
    };

    let Some(cfg) = intf_cfg else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let parts: Vec<&str> = asset_path.splitn(2, '/').collect();
    let key = parts[0];
    let subpath = parts.get(1).copied();

    let Some(asset_val) = cfg.assets.get(key) else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let p = libvellum::utils::expand_path(asset_val);
    let resolved = if p.is_absolute() { p } else { config_dir.join(p) };
    let Ok(resolved_canon) = resolved.canonicalize() else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let Ok(meta) = tokio::fs::metadata(&resolved_canon).await else {
        return StatusCode::NOT_FOUND.into_response();
    };

    let target_path = if meta.is_file() {
        if subpath.is_some() {
            return StatusCode::NOT_FOUND.into_response();
        }
        resolved_canon
    } else if meta.is_dir() {
        let Some(sub) = subpath else {
            return StatusCode::NOT_FOUND.into_response();
        };
        let full_path = resolved_canon.join(sub);
        let Ok(full_canon) = full_path.canonicalize() else {
            return StatusCode::NOT_FOUND.into_response();
        };
        if !full_canon.starts_with(&resolved_canon) || !full_canon.is_file() {
            return StatusCode::FORBIDDEN.into_response();
        }
        full_canon
    } else {
        return StatusCode::NOT_FOUND.into_response();
    };

    if let Ok(mut file) = tokio::fs::File::open(&target_path).await {
        let mut buf = Vec::new();
        if tokio::io::AsyncReadExt::read_to_end(&mut file, &mut buf).await.is_ok() {
            let mime = match target_path.extension().and_then(|e| e.to_str()) {
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
        let mut query = state.query.write().await;
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
        let mut query = state.query.write().await;
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
                let query = state.query.read().await;
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
                let query = state.query.read().await;
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
            let mut query = state.query.write().await;
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
    let query = state.query.read().await;
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
