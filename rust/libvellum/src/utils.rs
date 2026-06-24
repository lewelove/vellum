use std::path::PathBuf;
use base64::{Engine as _, engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}};

#[must_use]
pub fn expand_path(path_str: &str) -> PathBuf {
    if path_str.starts_with('~')
        && let Some(home) = dirs::home_dir() {
            if path_str == "~" {
                return home;
            }
            if let Some(stripped) = path_str.strip_prefix("~/") {
                return home.join(stripped);
            }
        }
    PathBuf::from(path_str)
}

pub fn get_file_info(path: &std::path::Path, rel_path: &str, compute_hash: bool) -> Result<serde_json::Value, anyhow::Error> {
    let m = std::fs::metadata(path)?;
    let mtime = m.modified()?.duration_since(std::time::SystemTime::UNIX_EPOCH)?.as_secs();
    let byte_size = m.len();
    
    let hash_val = if compute_hash {
        let content = std::fs::read(path)?;
        let hash = blake3::hash(&content);
        let raw = hash.as_bytes();
        let b64 = STANDARD.encode(raw);
        let string = format!("blake3-{b64}");
        let b64_url = URL_SAFE_NO_PAD.encode(raw);
        let address: String = b64_url.chars().take(16).collect();
        serde_json::json!({
            "string": string,
            "address": address
        })
    } else {
        serde_json::Value::Null
    };
    
    Ok(serde_json::json!({
        "path": rel_path,
        "hash": hash_val,
        "mtime": mtime,
        "byte_size": byte_size
    }))
}

#[must_use]
pub fn calculate_blake3_address(content: &[u8]) -> String {
    let hash = blake3::hash(content);
    let raw = hash.as_bytes();
    URL_SAFE_NO_PAD.encode(raw).chars().take(16).collect()
}
