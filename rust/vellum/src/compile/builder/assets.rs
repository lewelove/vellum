use crate::expand_path;
pub use fast_image_resize::FilterType;
use fast_image_resize::{ResizeAlg, ResizeOptions, Resizer};
use fast_image_resize::PixelType;
use fast_image_resize::images::Image;
use image::DynamicImage;
use serde_json::Value;
use std::path::{Path, PathBuf};

pub const COVER_CANDIDATES: [&str; 4] = ["cover.jpg", "cover.png", "folder.jpg", "front.jpg"];

pub fn resolve_cover_info(root: &Path) -> Option<PathBuf> {
    for c in COVER_CANDIDATES {
        let p = root.join(c);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

pub fn parse_interpolation(algo: &str) -> FilterType {
    match algo.to_lowercase().as_str() {
        "mitchell" => FilterType::Mitchell,
        "bilinear" => FilterType::Bilinear,
        "box" => FilterType::Box,
        "hamming" => FilterType::Hamming,
        "catmullrom" => FilterType::CatmullRom,
        _ => FilterType::Lanczos3,
    }
}

pub fn resize_image(src: &image::RgbImage, target_size: u32, filter: FilterType) -> Option<image::RgbImage> {
    let src_width = src.width();
    let src_height = src.height();
    let min_dim = std::cmp::min(src_width, src_height);

    let src_image = Image::from_vec_u8(
        src_width,
        src_height,
        src.clone().into_raw(),
        PixelType::U8x3,
    ).ok()?;

    let mut dst_image = Image::new(
        target_size,
        target_size,
        PixelType::U8x3,
    );

    let mut resizer = Resizer::new();
    let options = ResizeOptions::new()
        .crop(
            f64::from((src_width - min_dim) / 2),
            f64::from((src_height - min_dim) / 2),
            f64::from(min_dim),
            f64::from(min_dim),
        )
        .resize_alg(ResizeAlg::Convolution(filter));

    resizer.resize(&src_image, &mut dst_image, &options).ok()?;

    image::RgbImage::from_raw(target_size, target_size, dst_image.into_vec())
}

pub fn pregenerate_covers(
    config: &Value,
    cover_path: Option<&Path>,
    cover_hash_address: &str,
) -> Option<DynamicImage> {
    let storage = config.get("storage")?;
    let cache_str = storage.get("cache").and_then(Value::as_str)?;
    let original_path = cover_path?;
    if cover_hash_address.is_empty() {
        return None;
    }

    let cache_root = expand_path(cache_str);

    let covers_obj = config.get("compiler").and_then(|c| c.get("covers")).and_then(Value::as_object)?;

    let master_cfg = covers_obj.get("master")?;
    let master_size = master_cfg.get("size").and_then(Value::as_u64).unwrap_or(1080) as u32;
    let master_algo_str = master_cfg.get("interpolation").and_then(Value::as_str).unwrap_or("mitchell");
    let master_algo = parse_interpolation(master_algo_str);

    let master_qoi_path = cache_root
        .join("covers")
        .join("master")
        .join(master_algo_str)
        .join(format!("{master_size}px"))
        .join(format!("{cover_hash_address}.qoi"));

    if !master_qoi_path.exists() && let Ok(img) = image::open(original_path) {
        let img_rgb = img.to_rgb8();
        if let Some(parent) = master_qoi_path.parent() {
            std::fs::create_dir_all(parent).ok()?;
        }
        if let Some(resized) = resize_image(&img_rgb, master_size, master_algo) {
            resized.save_with_format(&master_qoi_path, image::ImageFormat::Qoi).ok();
        } else {
            img_rgb.save_with_format(&master_qoi_path, image::ImageFormat::Qoi).ok();
        }
    }

    let mut master_img: Option<image::RgbImage> = None;

    for (key, cfg) in covers_obj {
        if key == "master" {
            continue;
        }
        let target_size = cfg.get("size").and_then(Value::as_u64).unwrap_or(190) as u32;
        let algo_str = cfg.get("interpolation").and_then(Value::as_str).unwrap_or("lanczos");
        let algo = parse_interpolation(algo_str);

        let static_path = cache_root
            .join("covers")
            .join("static")
            .join(algo_str)
            .join(format!("{target_size}px"))
            .join(format!("{cover_hash_address}.qoi"));

        if !static_path.exists() {
            if master_img.is_none() {
                master_img = image::open(&master_qoi_path).ok().map(image::DynamicImage::into_rgb8);
            }
            if let Some(m_img) = &master_img {
                if let Some(parent) = static_path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                if let Some(resized) = resize_image(m_img, target_size, algo) {
                    resized.save_with_format(&static_path, image::ImageFormat::Qoi).ok();
                }
            }
        }
    }

    master_img.map_or_else(
        || image::open(&master_qoi_path).ok(),
        |m| Some(DynamicImage::ImageRgb8(m)),
    )
}
