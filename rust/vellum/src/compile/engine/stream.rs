use crate::compile::{ExportTarget, builder, engine::verify};
use libvellum::error::VellumError;
use anyhow::Result;
use rayon::prelude::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct AlbumUpdateSignal {
    pub path: PathBuf,
    pub artist: String,
    pub album: String,
}

pub struct StreamContext {
    pub albums: Vec<PathBuf>,
    pub config: Arc<libvellum::lua::ResolvedConfig>,
    pub target: ExportTarget,
    pub jobs: Option<usize>,
    pub notify_tx: Option<mpsc::Sender<AlbumUpdateSignal>>,
}

pub async fn run(ctx: StreamContext) -> Result<()> {
    let (dtx, mut drx) = mpsc::channel::<Value>(512);

    let notify = ctx.notify_tx.clone().map(Arc::new);
    let build_handle = spawn_builders(&ctx, dtx);

    let d_notify = notify.clone();
    let target = ctx.target;
    let direct_handle = tokio::spawn(async move {
        while let Some(v) = drx.recv().await {
            let n = d_notify.as_ref().map(Arc::clone);
            tokio::task::spawn_blocking(move || {
                let _ = finalize(v, target, n);
            });
        }
    });

    let _ = build_handle.await;
    let _ = direct_handle.await;
    Ok(())
}

fn spawn_builders(
    ctx: &StreamContext,
    dtx: mpsc::Sender<Value>,
) -> tokio::task::JoinHandle<()> {
    let albums = ctx.albums.clone();
    let cfg = Arc::clone(&ctx.config);
    let jobs = ctx.jobs;

    tokio::task::spawn_blocking(move || {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs.unwrap_or(1))
            .build()
            .unwrap();
        pool.install(|| {
            albums.par_iter().for_each(|ar| {
                match builder::build(ar, &cfg) {
                    Ok(man) => {
                        let _ = dtx.blocking_send(man);
                    }
                    Err(e) => match e {
                        VellumError::ManifestIoError(_) 
                        | VellumError::ManifestParseError { .. }
                        | VellumError::JsonError(_) 
                        | VellumError::MissingCompilerRegistry => {
                            log::error!("SYSTEM FAILURE: {e}");
                        }
                        _ => {
                            log::warn!("VALIDATION REJECTED: {e}");
                        }
                    }
                }
            });
        });
    })
}

fn strip_empty_values(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.retain(|_, v| match v {
                Value::String(s) => !s.is_empty(),
                Value::Null => false,
                _ => true,
            });
            for v in map.values_mut() {
                strip_empty_values(v);
            }
        }
        Value::Array(arr) => {
            for v in arr.iter_mut() {
                strip_empty_values(v);
            }
        }
        _ => {}
    }
}

fn format_json_value(value: &Value, indent: usize, out: &mut String) {
    let pad = "  ".repeat(indent);
    match value {
        Value::Object(map) => {
            if map.is_empty() {
                out.push_str("{}");
                return;
            }
            out.push_str("{\n");
            let mut it = map.iter().peekable();
            while let Some((k, v)) = it.next() {
                out.push_str(&pad);
                out.push_str("  ");
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push_str(": ");
                format_json_value(v, indent + 1, out);
                if it.peek().is_some() {
                    out.push_str(",\n");
                } else {
                    out.push('\n');
                }
            }
            out.push_str(&pad);
            out.push('}');
        }
        Value::Array(arr) => {
            if arr.is_empty() {
                out.push_str("[]");
                return;
            }

            let is_simple = arr.iter().all(|v| !matches!(v, Value::Object(_) | Value::Array(_)));

            if is_simple {
                out.push('[');
                let mut it = arr.iter().peekable();
                while let Some(v) = it.next() {
                    format_json_inline(v, out);
                    if it.peek().is_some() {
                        out.push_str(", ");
                    }
                }
            } else {
                out.push_str("[\n");
                let mut it = arr.iter().peekable();
                while let Some(v) = it.next() {
                    out.push_str(&pad);
                    out.push_str("  ");
                    format_json_value(v, indent + 1, out);
                    if it.peek().is_some() {
                        out.push_str(",\n");
                    } else {
                        out.push('\n');
                    }
                }
                out.push_str(&pad);
            }
            out.push(']');
        }
        _ => {
            out.push_str(&serde_json::to_string(value).unwrap_or_default());
        }
    }
}

fn format_json_inline(value: &Value, out: &mut String) {
    if let Value::Array(arr) = value {
        out.push('[');
        let mut it = arr.iter().peekable();
        while let Some(v) = it.next() {
            format_json_inline(v, out);
            if it.peek().is_some() {
                out.push_str(", ");
            }
        }
        out.push(']');
    } else if let Value::Object(_) = value {
        out.push_str("{}");
    } else {
        out.push_str(&serde_json::to_string(value).unwrap_or_default());
    }
}

fn finalize(
    mut v: Value,
    target: ExportTarget,
    notify_tx: Option<Arc<mpsc::Sender<AlbumUpdateSignal>>>,
) -> Result<()> {
    let artist = v.get("album").and_then(|a| a.get("albumartist")).and_then(Value::as_str).unwrap_or("Unknown").to_string();
    let album = v.get("album").and_then(|a| a.get("album")).and_then(Value::as_str).unwrap_or("Unknown").to_string();

    let ctx = v
        .as_object_mut()
        .and_then(|o| o.remove("ctx"))
        .unwrap_or_else(|| json!({}));
    let harvest = ctx.get("harvest").cloned().unwrap_or_else(|| json!([]));
    let h_arr = harvest.as_array().map_or(&[][..], Vec::as_slice);

    let default_keys = vec![
        "album".to_string(),
        "albumartist".to_string(),
        "date".to_string(),
        "genre".to_string(),
        "comment".to_string(),
        "title".to_string(),
        "artist".to_string(),
        "tracknumber".to_string(),
        "discnumber".to_string(),
    ];

    verify::calculate_file_tag_subset_match(&mut v, h_arr, &default_keys);

    strip_empty_values(&mut v);

    let album_root_str = ctx
        .get("paths")
        .and_then(|p| p.get("album_root"))
        .and_then(Value::as_str);

    if let Some(path) = album_root_str {
        let album_root = Path::new(path);

        let mut content = String::new();
        format_json_value(&v, 0, &mut content);

        if target == ExportTarget::Stdout {
            println!("{content}");
        } else {
            std::fs::write(album_root.join("album.lock.json"), content)?;
            if let Some(tx_arc) = notify_tx {
                let root_clone = album_root.to_path_buf();
                let tx = (*tx_arc).clone();
                tokio::spawn(async move {
                    let _ = tx.send(AlbumUpdateSignal {
                        path: root_clone,
                        artist,
                        album,
                    }).await;
                });
            }
        }
    }
    Ok(())
}
