use crate::server::state::AppState;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use std::path::{Path as StdPath, PathBuf};
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

fn find_cached_cover(cache_root: &StdPath, algo: &str, width: u32, hash: &str) -> Option<PathBuf> {
    let static_path = cache_root
        .join("covers")
        .join("static")
        .join(algo)
        .join(format!("{width}px"))
        .join(format!("{hash}.qoi"));

    let dynamic_path = cache_root
        .join("covers")
        .join("dynamic")
        .join(algo)
        .join(format!("{width}px"))
        .join(format!("{hash}.qoi"));

    let master_path = cache_root
        .join("covers")
        .join("master")
        .join(algo)
        .join(format!("{width}px"))
        .join(format!("{hash}.qoi"));

    if static_path.exists() {
        Some(static_path)
    } else if dynamic_path.exists() {
        Some(dynamic_path)
    } else if master_path.exists() {
        Some(master_path)
    } else {
        None
    }
}

async fn load_image_bmp(path: PathBuf) -> Option<Vec<u8>> {
    tokio::task::spawn_blocking(move || {
        let img = image::open(&path).ok()?.into_rgb8();
        let mut buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut buf);
        img.write_to(&mut cursor, image::ImageFormat::Bmp).ok()?;
        Some(buf)
    })
    .await
    .ok()?
}

async fn ensure_master_cover(
    state: &Arc<AppState>,
    library_root: &StdPath,
    master_path: PathBuf,
    master_size: u32,
    master_algo_str: &str,
    hash: &str,
) -> Result<(), StatusCode> {
    if master_path.exists() {
        return Ok(());
    }

    let source_info = {
        let query = state.query.read().await;
        query.dict.values().find(|v| {
            v.get("cover_hash").and_then(|h| h.as_str()) == Some(hash)
        }).map(|v| {
            (
                v.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                v.get("cover_path").and_then(|p| p.as_str()).unwrap_or("cover.jpg").to_string()
            )
        })
    };

    let Some((album_id, cover_path)) = source_info else {
        return Err(StatusCode::NOT_FOUND);
    };

    let original_path = library_root.join(album_id).join(cover_path);
    let blob_path_clone = master_path.clone();
    let master_algo_str_clone = master_algo_str.to_string();

    let gen_result = tokio::task::spawn_blocking(move || {
        let img = image::open(&original_path).ok()?;
        let img_rgb = img.into_rgb8();
        let filter = crate::compile::assets::parse_interpolation(&master_algo_str_clone);
        if let Some(parent) = blob_path_clone.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        if let Some(resized) = crate::compile::assets::resize_image(&img_rgb, master_size, filter) {
            resized.save_with_format(&blob_path_clone, image::ImageFormat::Qoi).ok()?;
        } else {
            img_rgb.save_with_format(&blob_path_clone, image::ImageFormat::Qoi).ok()?;
        }
        Some(())
    }).await;

    if gen_result.is_err() || gen_result.unwrap().is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    Ok(())
}

async fn create_resized_dynamic(
    master_path: PathBuf,
    dynamic_path: PathBuf,
    algo: String,
    width: u32,
) -> Result<Vec<u8>, StatusCode> {
    let result = tokio::task::spawn_blocking(move || {
        let img = image::open(&master_path).ok()?.into_rgb8();
        let filter = crate::compile::assets::parse_interpolation(&algo);
        let resized = crate::compile::assets::resize_image(&img, width, filter)?;
        let mut bmp_buf = Vec::new();
        let mut cursor = std::io::Cursor::new(&mut bmp_buf);
        resized.write_to(&mut cursor, image::ImageFormat::Bmp).ok()?;
        if let Some(parent) = dynamic_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }
        resized.save_with_format(&dynamic_path, image::ImageFormat::Qoi).ok();
        Some(bmp_buf)
    }).await;

    match result {
        Ok(Some(buf)) => Ok(buf),
        _ => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

pub async fn get_resized_cover(
    Path((algo, size_str, hash)): Path<(String, String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let width = size_str
        .strip_suffix("px")
        .unwrap_or(&size_str)
        .parse::<u32>()
        .unwrap_or(200)
        .clamp(16, 2048);

    let (cache_root, library_root) = {
        let guard = state.config.read().await;
        let cache = guard.cache_root.clone();
        let library = guard.library_root.clone();
        drop(guard);
        (cache, library)
    };

    if let Some(t_path) = find_cached_cover(&cache_root, &algo, width, &hash)
        && let Some(buf) = load_image_bmp(t_path).await {
            return ([
                (header::CONTENT_TYPE, HeaderValue::from_static("image/bmp")),
                (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000, immutable")),
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
            ], buf).into_response();
        }

    let (master_size, master_algo_str) = {
        let guard = state.config.read().await;
        let size = guard.covers.get("master").map_or(1080, |c| c.size);
        let algo_str = guard.covers.get("master").and_then(|c| c.interpolation.clone()).unwrap_or_else(|| "mitchell".to_string());
        drop(guard);
        (size, algo_str)
    };

    let master_blob_path = cache_root
        .join("covers")
        .join("master")
        .join(&master_algo_str)
        .join(format!("{master_size}px"))
        .join(format!("{hash}.qoi"));

    if let Err(status) = ensure_master_cover(&state, &library_root, master_blob_path.clone(), master_size, &master_algo_str, &hash).await {
        return status.into_response();
    }

    let dynamic_path = cache_root
        .join("covers")
        .join("dynamic")
        .join(&algo)
        .join(format!("{width}px"))
        .join(format!("{hash}.qoi"));

    match create_resized_dynamic(master_blob_path, dynamic_path, algo, width).await {
        Ok(buf) => ([
            (header::CONTENT_TYPE, HeaderValue::from_static("image/bmp")),
            (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=31536000, immutable")),
            (header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
        ], buf).into_response(),
        Err(status) => status.into_response(),
    }
}

pub async fn get_album_metadata(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let json_str = {
        let query = state.query.read().await;
        query.get_album_json(&id)
    };
    if let Some(data) = json_str {
        return ([(header::CONTENT_TYPE, "application/json")],
            data
        ).into_response();
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn get_album_cover(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let path_opt = {
        let query = state.query.read().await;
        let config_guard = state.config.read().await;
        
        query.dict.get(&id).map(|meta| {
            let cp = meta.get("cover_path").and_then(|v| v.as_str()).unwrap_or("default_cover.png");
            config_guard.library_root.join(&id).join(cp)
        })
    };

    if let Some(path) = path_opt {
        return serve_image(path, false).await;
    }
    StatusCode::NOT_FOUND.into_response()
}

pub async fn get_lyrics(
    Path((id, path)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let full_path = {
        let config_guard = state.config.read().await;
        config_guard.library_root.join(&id).join(&path)
    };

    if full_path.exists()
        && full_path.is_file()
        && let Ok(mut file) = File::open(&full_path).await
    {
        let mut buf = String::new();
        if file.read_to_string(&mut buf).await.is_ok() {
            return ([
                    (
                        header::CONTENT_TYPE,
                        HeaderValue::from_static("text/plain; charset=utf-8"),
                    ),
                    (header::CACHE_CONTROL, HeaderValue::from_static("no-cache")),
                ],
                buf,
            )
                .into_response();
        }
    }

    StatusCode::NOT_FOUND.into_response()
}

async fn serve_image(path: PathBuf, is_immutable: bool) -> Response {
    if let Ok(mut file) = File::open(&path).await {
        let mut buf = Vec::new();
        if file.read_to_end(&mut buf).await.is_ok() {
            let mime = if path.extension().is_some_and(|e| e == "png") {
                "image/png"
            } else {
                "image/jpeg"
            };
            
            let cache_header = if is_immutable {
                HeaderValue::from_static("public, max-age=31536000, immutable")
            } else {
                HeaderValue::from_static("public, max-age=86400")
            };

            return ([
                    (header::CONTENT_TYPE, HeaderValue::from_static(mime)),
                    (header::CACHE_CONTROL, cache_header),
                    (
                        header::ACCESS_CONTROL_ALLOW_ORIGIN,
                        HeaderValue::from_static("*"),
                    ),
                ],
                buf,
            )
                .into_response();
        }
    }
    StatusCode::NOT_FOUND.into_response()
}
