use crate::compile::assets;
use libvellum::models::CoverMetrics;
use serde_json::{Value, json};
use std::path::Path;
use base64::{Engine as _, engine::general_purpose::STANDARD};

pub fn resolve_cover_data(
    album_root: &Path,
    config: &libvellum::lua::ResolvedConfig,
) -> (Value, Option<CoverMetrics>) {
    let main_cover_path = assets::resolve_cover_info(album_root);
    
    let mut cover_hash_address = String::new();
    let mut cover_file_info = Value::Null;

    if let Some(cp) = &main_cover_path {
        let content = std::fs::read(cp).unwrap_or_default();
        if !content.is_empty() {
            cover_hash_address = libvellum::utils::calculate_blake3_address(&content);
            let raw = blake3::hash(&content);
            let cover_hash_full = format!("blake3-{}", STANDARD.encode(raw.as_bytes()));
            let rel_path = libvellum::resolvers::rel_path(cp, album_root);
            if let Ok(info) = libvellum::utils::get_file_info(cp, &rel_path, false) {
                let mut info_map = info.as_object().unwrap().clone();
                info_map.insert("address".to_string(), json!(cover_hash_address));
                info_map.insert("hash".to_string(), json!(cover_hash_full));
                cover_file_info = Value::Object(info_map);
            }
        }
    }

    let loaded_image = assets::pregenerate_covers(config, main_cover_path.as_deref(), &cover_hash_address);
    let cover_metrics = resolve_cover_metrics(config, &cover_hash_address, loaded_image.as_ref());

    (cover_file_info, cover_metrics)
}

pub fn resolve_cover_metrics(
    config: &libvellum::lua::ResolvedConfig,
    c_hash: &str,
    loaded_image: Option<&image::DynamicImage>,
) -> Option<CoverMetrics> {
    if c_hash.is_empty() {
        return None;
    }
    
    let cache_str = &config.app.storage.cache;
    let cache_root = libvellum::utils::expand_path(cache_str);
    let metrics_dir = cache_root.join("cover_data");
    std::fs::create_dir_all(&metrics_dir).ok();
    
    let metrics_path = metrics_dir.join(format!("{c_hash}.json"));
    
    let mut metrics = if metrics_path.exists() {
        std::fs::read_to_string(&metrics_path).map_or(None, |content| serde_json::from_str::<CoverMetrics>(&content).ok())
    } else { 
        None 
    }.unwrap_or_else(|| CoverMetrics {
        hash: c_hash.to_string(),
        entropy: None,
        chroma: None,
    });
    
    let mut needs_save = false;
    
    if let Some(img) = loaded_image {
        if metrics.chroma.is_none() {
            metrics.chroma = Some(libvellum::images::cover_chroma::calculate_chroma(img));
            needs_save = true;
        }
        if metrics.entropy.is_none() {
            metrics.entropy = Some(libvellum::images::cover_entropy::calculate_entropy(img));
            needs_save = true;
        }
    }
    
    if needs_save
        && let Ok(content) = serde_json::to_string(&metrics) {
            let _ = std::fs::write(&metrics_path, content);
        }
    
    Some(metrics)
}
