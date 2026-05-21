use crate::compile::builder::assets::generate_master_blob;
use crate::server::state::AppState;
use axum::extract::{Path, State};
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use fast_image_resize::images::Image;
use fast_image_resize::{FilterType, ResizeAlg, ResizeOptions, Resizer};
use fast_image_resize::PixelType;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

pub async fn get_resized_cover(
    Path((width_str, hash)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let width = width_str
        .strip_suffix("px")
        .unwrap_or(&width_str)
        .parse::<u32>()
        .unwrap_or(200)
        .clamp(16, 2048);

    let (cache_root, library_root) = {
        let guard = state.config.read().await;
        (guard.cache_root.clone(), guard.library_root.clone())
    };
    
    let master_blob_path = cache_root.join("covers").join(format!("{hash}.bmp"));

    if !master_blob_path.exists() {
        let source_info = {
            let query = state.query.lock().await;
            query.dict.values().find(|v| {
                v.get("cover_hash").and_then(|h| h.as_str()) == Some(&hash)
            }).map(|v| {
                (
                    v.get("id").and_then(|i| i.as_str()).unwrap_or("").to_string(),
                    v.get("cover_path").and_then(|p| p.as_str()).unwrap_or("cover.jpg").to_string()
                )
            })
        };

        if let Some((album_id, cover_path)) = source_info {
            let original_path = library_root.join(album_id).join(cover_path);
            let blob_path_clone = master_blob_path.clone();

            let gen_result = tokio::task::spawn_blocking(move || {
                generate_master_blob(&original_path, &blob_path_clone)
            }).await;

            if gen_result.is_err() || gen_result.unwrap().is_err() {
                return StatusCode::NOT_FOUND.into_response();
            }
        } else {
            return StatusCode::NOT_FOUND.into_response();
        }
    }

    let result = tokio::task::spawn_blocking(move || {
        let img = image::open(&master_blob_path).ok()?.into_rgb8();
        let src_width = img.width();
        let src_height = img.height();
        let min_dim = std::cmp::min(src_width, src_height);

        let src_image = Image::from_vec_u8(src_width, src_height, img.into_raw(), PixelType::U8x3).ok()?;
        let mut dst_image = Image::new(width, width, PixelType::U8x3);
        let mut resizer = Resizer::new();
        let options = ResizeOptions::new()
            .crop(
                f64::from((src_width - min_dim) / 2),
                f64::from((src_height - min_dim) / 2),
                f64::from(min_dim),
                f64::from(min_dim),
            )
            .resize_alg(ResizeAlg::Convolution(FilterType::Mitchell));

        resizer.resize(&src_image, &mut dst_image, &options).ok()?;

        let mut buf = Vec::with_capacity((width * width * 3 + 54) as usize);
        let img_buffer = image::RgbImage::from_raw(width, width, dst_image.into_vec())?;
        let mut cursor = std::io::Cursor::new(&mut buf);
        img_buffer.write_to(&mut cursor, image::ImageFormat::Bmp).ok()?;
        
        Some(buf)
    }).await;

    match result {
        Ok(Some(buf)) => {
            ([
                (header::CONTENT_TYPE, HeaderValue::from_static("image/bmp")),
                (header::CACHE_CONTROL, HeaderValue::from_static("public, max-age=3600")),
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, HeaderValue::from_static("*")),
            ], buf).into_response()
        },
        _ => StatusCode::INTERNAL_SERVER_ERROR.into_response()
    }
}

pub async fn get_cover_thumbnail(
    Path((size, hash)): Path<(String, String)>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let root = {
        let guard = state.config.read().await;
        guard.cache_root.clone()
    };
    
    let path = root.join("thumbnails").join(&size).join(format!("{hash}.png"));

    match serve_image(path.clone(), true).await {
        resp if resp.status() == StatusCode::OK => resp,
        _ => {
            log::error!("FS 404: File not found at -> {}", path.display());
            StatusCode::NOT_FOUND.into_response()
        }
    }
}

pub async fn get_album_metadata(
    Path(id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> Response {
    let json_str = {
        let query = state.query.lock().await;
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
        let query = state.query.lock().await;
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

pub async fn get_custom_shader(State(state): State<Arc<AppState>>) -> Response {
    let path_opt = {
        let guard = state.config.read().await;
        guard.resolved_shader_path.clone()
    };

    if let Some(path) = path_opt
        && let Ok(mut file) = File::open(&path).await {
            let mut buf = String::new();
            if file.read_to_string(&mut buf).await.is_ok() {
                return ([
                        (
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/x-glsl; charset=utf-8"),
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

pub async fn get_custom_css(State(state): State<Arc<AppState>>) -> Response {
    let path_opt = {
        let guard = state.config.read().await;
        guard.resolved_css_path.clone()
    };

    if let Some(path) = path_opt
        && let Ok(mut file) = File::open(&path).await {
            let mut buf = String::new();
            if file.read_to_string(&mut buf).await.is_ok() {
                return ([
                        (
                            header::CONTENT_TYPE,
                            HeaderValue::from_static("text/css; charset=utf-8"),
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
