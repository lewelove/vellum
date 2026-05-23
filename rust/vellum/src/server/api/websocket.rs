use crate::server::mpd::MpdCommand;
use crate::server::state::AppState;
use ax_ws::WebSocket;
use axum::extract::ws as ax_ws;
use axum::extract::{State, WebSocketUpgrade};
use axum::response::Response;
use serde_json::json;
use std::sync::Arc;

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<Arc<AppState>>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<AppState>) {
    log::info!("Client connected");

    let init_payload = {
        let (dict, track_map, manifest) = {
            let q = state.query.lock().await;
            (q.dict.clone(), q.track_lookup.clone(), q.manifest.clone())
        };
        let ui_data = state.ui_state.read().await.clone();
        let (covers, shader) = {
            let c = state.config.read().await;
            (c.covers.clone(), c.shader.clone())
        };
        
        json!({
            "type": "INIT_DICT",
            "dict": dict,
            "trackMap": track_map,
            "manifest": manifest,
            "ui_state": ui_data,
            "config": {
                "covers": covers,
                "shader": shader
            }
        })
        .to_string()
    };

    if socket
        .send(ax_ws::Message::Text(init_payload.into()))
        .await
        .is_err()
    {
        return;
    }

    state.mpd_engine.send(MpdCommand::Refresh).await;

    let mut rx = state.tx.subscribe();
    loop {
        tokio::select! {
            Some(msg) = socket.recv() => {
                match msg {
                    Ok(ax_ws::Message::Text(text)) => {
                        if let Ok(req) = serde_json::from_str::<serde_json::Value>(&text) {
                            let req_type = req.get("type").and_then(|v| v.as_str()).unwrap_or("");
                            if req_type == "VIEW_REQUEST" {
                                let collection = req.get("collection").and_then(|v| v.as_str()).unwrap_or("library");
                                let sort = req.get("sort").and_then(|v| v.as_str()).unwrap_or("default");
                                let reverse = req.get("reverse").and_then(serde_json::Value::as_bool).unwrap_or(false);
                                let filter_key = req.get("filter").and_then(|v| v.get("key")).and_then(|v| v.as_str());
                                let filter_val = req.get("filter").and_then(|v| v.get("val")).and_then(|v| v.as_str());
                                
                                let ids = state.query.lock().await.request_view(collection, sort, filter_key, filter_val, reverse);
                                let _ = socket.send(ax_ws::Message::Text(json!({ "type": "VIEW_DATA", "ids": ids }).to_string().into())).await;
                            } else if req_type == "SHELF_REQUEST" {
                                let shelf = req.get("shelf").and_then(|v| v.as_str()).unwrap_or("");
                                let ids = state.query.lock().await.request_shelf_view(shelf);
                                let _ = socket.send(ax_ws::Message::Text(json!({ "type": "VIEW_DATA", "ids": ids }).to_string().into())).await;
                            } else if req_type == "GROUP_REQUEST" {
                                let collection = req.get("collection").and_then(|v| v.as_str()).unwrap_or("library");
                                let key = req.get("key").and_then(|v| v.as_str()).unwrap_or("");
                                
                                let result = state.query.lock().await.request_group(collection, key);
                                let _ = socket.send(ax_ws::Message::Text(json!({ "type": "GROUP_RESULT", "key": key, "result": result }).to_string().into())).await;
                            }
                        }
                    }
                    Ok(ax_ws::Message::Close(_)) | Err(_) => {
                        log::info!("Client disconnected");
                        break;
                    }
                    _ => {}
                }
            }
            Ok(msg) = rx.recv() => {
                if socket.send(ax_ws::Message::Text(msg.into())).await.is_err() {
                    break;
                }
            }
        }
    }
}
