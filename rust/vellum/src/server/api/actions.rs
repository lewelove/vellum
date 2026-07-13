use crate::server::state::AppState;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde_json::json;
use std::sync::Arc;
use std::collections::HashMap;

async fn resolve_target_ids(
    params: &HashMap<String, String>,
    state: &Arc<AppState>,
    library_root: &std::path::Path,
) -> Vec<String> {
    let mut target_ids = Vec::new();
    let playing = params.get("playing").is_some_and(|v| v == "true");
    let id_arg = params.get("id").cloned();
    let query_arg = params.get("query").cloned();
    let file_arg = params.get("file").cloned();

    if let Some(q) = query_arg {
        let query_guard = state.query.lock().await;
        let expanded = crate::server::query::expand_shorthand(&q);
        if let Ok(ids) = query_guard.query_ids(&expanded) {
            target_ids = ids;
        }
    } else if let Some(id) = id_arg {
        target_ids.push(id);
    } else if playing {
        if let Ok(path) = crate::x::get_playing_album(&library_root.to_string_lossy()).await
            && let Ok(rel) = path.strip_prefix(library_root)
        {
            target_ids.push(rel.to_string_lossy().to_string());
        }
    } else if let Some(f) = file_arg {
        let mut p = libvellum::utils::expand_path(&f);
        if p.is_dir() {
            p = p.join("album.lock.json");
        }
        if let Ok(content) = std::fs::read_to_string(&p)
            && let Ok(lock_json) = serde_json::from_str::<serde_json::Value>(&content)
            && let Some(id) = lock_json.pointer("/album/id").and_then(|v| v.as_str())
        {
            target_ids.push(id.to_string());
        }
    }
    
    target_ids
}

pub async fn execute_action(
    Path(name): Path<String>,
    Query(params): Query<HashMap<String, String>>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let name_key = name.replace('-', "_");

    let config_guard = state.config.read().await;
    let action_cfg_opt = config_guard.actions.get(&name_key).cloned();
    let library_root = config_guard.library_root.clone();
    let app_config_json = serde_json::to_value(&config_guard.app).unwrap_or_else(|_| json!({}));
    let config_dir = config_guard.config_dir.clone();
    let env_vars = crate::x::load_env_vars_from_path(config_guard.app.storage.environment.as_deref());
    drop(config_guard);

    let Some(action_cfg) = action_cfg_opt else {
        log::error!("Action '{name}' is not declared in configuration.");
        return (StatusCode::BAD_REQUEST, Json(json!({"error": format!("Action '{name}' is not declared in configuration.")}))).into_response();
    };

    let target_ids = resolve_target_ids(&params, &state, &library_root).await;

    let mut lock_jsons = Vec::new();
    for target_id in &target_ids {
        let lock_file_path = library_root.join(target_id).join("album.lock.json");
        if let Ok(json_data) = std::fs::read_to_string(&lock_file_path)
            && let Ok(lock_json) = serde_json::from_str::<serde_json::Value>(&json_data)
        {
            lock_jsons.push(lock_json);
        }
    }

    let mut options_vec = Vec::new();
    for (k, v) in &params {
        if k != "playing" && k != "id" && k != "query" && k != "file" {
            if v.is_empty() {
                options_vec.push(k.clone());
            } else {
                options_vec.push(format!("{k}={v}"));
            }
        }
    }
    let options_str = options_vec.join(" ");

    let combined_json = json!({
        "albums": lock_jsons,
        "config": {
            "vellum": app_config_json,
            "action": action_cfg.config
        },
        "options": options_str
    });

    let run_str = action_cfg.run.unwrap_or_else(|| format!("~/.local/share/vellum/actions/{name_key}.sh"));
    let expanded_action_path = libvellum::utils::expand_path(&run_str);
    let action_path = if expanded_action_path.is_absolute() {
        expanded_action_path
    } else {
        config_dir.join(expanded_action_path)
    };

    if !action_path.exists() {
        log::error!("Action path '{}' does not exist.", action_path.display());
        return (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": format!("Action path '{}' does not exist.", action_path.display())}))).into_response();
    }

    let cmd = if action_path.extension().is_some_and(|e| e == "py") {
        "python"
    } else if action_path.extension().is_some_and(|e| e == "sh") {
        "sh"
    } else {
        action_path.to_str().unwrap()
    };

    let mut command = tokio::process::Command::new(cmd);
    command.envs(&env_vars);
    if cmd == "python" || cmd == "sh" {
        command.arg(&action_path);
    }

    match command
        .stdin(std::process::Stdio::piped())
        .spawn()
    {
        Ok(mut child) => {
            if let Some(mut stdin) = child.stdin.take() {
                let payload = serde_json::to_string(&combined_json).unwrap_or_default();
                tokio::spawn(async move {
                    let _ = tokio::io::AsyncWriteExt::write_all(&mut stdin, payload.as_bytes()).await;
                });
            }
            tokio::spawn(async move {
                if let Ok(status) = child.wait().await {
                    if !status.success() {
                        log::error!("Action failed with status: {status}");
                    }
                } else {
                    log::error!("Failed to wait on action child process.");
                }
            });
            Json(json!({"status": "ok"})).into_response()
        }
        Err(e) => {
            log::error!("Failed to spawn action: {e}");
            (StatusCode::INTERNAL_SERVER_ERROR, Json(json!({"error": e.to_string()}))).into_response()
        }
    }
}
