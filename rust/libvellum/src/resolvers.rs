use serde_json::{Value, json};
use std::path::Path;
use crate::models::CoverMetrics;
use crate::error::VellumError;

pub fn resolve_genre(source: &Value, path: &Path) -> Result<Value, VellumError> {
    let mut list = crate::types::resolve_type_array(source, "genre", "", path)?;
    if let Value::Array(ref arr) = list
        && arr.is_empty()
    {
        list = json!(["Unknown"]);
    }
    Ok(list)
}

#[must_use]
pub fn resolve_tracknumber(val: &Value) -> u32 {
    match val {
        Value::Number(n) => n.as_u64().and_then(|i| u32::try_from(i).ok()).unwrap_or(0),
        Value::String(s) => {
            let base = s.split('/').next().unwrap_or("").trim();
            base.parse::<u32>().unwrap_or(0)
        }
        _ => 0,
    }
}

#[must_use]
pub fn resolve_discnumber(val: &Value) -> u32 {
    match val {
        Value::Number(n) => n.as_u64().and_then(|i| u32::try_from(i).ok()).unwrap_or(1),
        Value::String(s) => {
            let base = s.split('/').next().unwrap_or("").trim();
            base.parse::<u32>().unwrap_or(1)
        }
        _ => 1,
    }
}

pub fn resolve_comment(source: &Value, path: &Path) -> Result<String, VellumError> {
    if let Some(v) = source.get("comment").and_then(Value::as_str)
        && !v.is_empty()
    {
        return Ok(v.to_string());
    }

    let country = crate::types::resolve_type_string(source, "country", "", "", path)?.as_str().unwrap_or("").to_string();
    let label = crate::types::resolve_type_string(source, "label", "", "", path)?.as_str().unwrap_or("").to_string();
    let cat = crate::types::resolve_type_string(source, "catalognumber", "", "", path)?.as_str().unwrap_or("").to_string();
    if country.is_empty() && label.is_empty() && cat.is_empty() {
        return Ok(String::new());
    }
    let dt = resolve_date(source, path)?;
    let year = if dt.len() >= 4 {
        &dt[0..4]
    } else {
        ""
    };
    
    let parts = [
        year,
        &country,
        &label,
        &cat,
    ];

    Ok(parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" "))
}

pub fn resolve_date(source: &Value, path: &Path) -> Result<String, VellumError> {
    if let Some(v) = source.get("release_date").and_then(Value::as_str) {
        return Ok(v.to_string());
    }
    let d = crate::types::resolve_type_string(source, "date", "year,originalyear", "0000", path)?.as_str().unwrap_or("0000").to_string();
    if d.len() >= 4 {
        Ok(format!("{}-00", &d[0..4]))
    } else {
        Ok("0000-00".to_string())
    }
}

pub fn resolve_original_date(source: &Value, path: &Path) -> Result<String, VellumError> {
    resolve_date(source, path)
}

pub fn resolve_release_date(source: &Value, path: &Path) -> Result<String, VellumError> {
    resolve_date(source, path)
}

pub fn resolve_album_info_date_added(album_root: &Path, source: &Value, config: &crate::lua::ResolvedConfig) -> Result<String, VellumError> {
    let local_toml_path = album_root.join("local.toml");
    if local_toml_path.exists()
        && let Ok(content) = std::fs::read_to_string(&local_toml_path)
        && let Ok(parsed) = toml::from_str::<toml::Value>(&content)
        && let Some(lib) = parsed.get("local")
        && let Some(da) = lib.get("date_added")
    {
        let dt_str = match da {
            toml::Value::Datetime(dt) => dt.to_string(),
            toml::Value::String(s) => s.clone(),
            _ => String::new(),
        };
        let json_val = Value::String(dt_str);
        return Ok(crate::types::parse_time(Some(&json_val)));
    }

    if let Some(fallbacks) = &config.app.compiler.date_added {
        for f in fallbacks {
            if let Some(val) = source.get(f) {
                return Ok(crate::types::parse_time(Some(val)));
            }
        }
    }

    Ok(crate::types::parse_time(None))
}

#[must_use]
pub fn resolve_lyrics_path(
    album_root: &Path,
    track_num: u32,
    disc_num: u32,
    total_discs: u32,
) -> Option<String> {
    let folders = [
        "lyrics",
        "Lyrics",
    ];
    let mut candidates = Vec::new();

    for folder in folders {
        let dir = album_root.join(folder);
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let Some(ext) = path
                .extension()
                .and_then(|e| e.to_str())
                .map(str::to_lowercase)
            else {
                continue;
            };
            if ext != "lrc" && ext != "txt" {
                continue;
            }

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            let is_match = if total_discs > 1 {
                name.find('.').is_some_and(|dot_idx| {
                    let disc_part = &name[..dot_idx];
                    let remaining = &name[dot_idx + 1..];
                    let track_part = remaining
                        .chars()
                        .take_while(char::is_ascii_digit)
                        .collect::<String>();

                    let d_match = disc_part.parse::<u32>().is_ok_and(|d| d == disc_num);
                    let t_match = track_part.parse::<u32>().is_ok_and(|t| t == track_num);
                    d_match && t_match
                })
            } else {
                let track_part = name
                    .chars()
                    .take_while(char::is_ascii_digit)
                    .collect::<String>();
                track_part.parse::<u32>().is_ok_and(|t| t == track_num)
            };

            if is_match {
                candidates.push((rel_path(&path, album_root), ext));
            }
        }
    }

    if candidates.is_empty() {
        return None;
    }

    if let Some(lrc) = candidates.iter().find(|(_, ext)| ext == "lrc") {
        return Some(lrc.0.clone());
    }

    candidates.first().map(|(path, _)| path.clone())
}

#[must_use]
pub fn resolve_cover_chroma(cover_metrics: Option<&CoverMetrics>) -> Option<Value> {
    cover_metrics.and_then(|m| m.chroma).map(|c| json!(c))
}

#[must_use]
pub fn resolve_cover_entropy(cover_metrics: Option<&CoverMetrics>) -> Option<Value> {
    cover_metrics.and_then(|m| m.entropy).map(|e| json!(e))
}

#[must_use]
pub fn calculate_total_discs(tracks: &[Value]) -> u32 {
    let mut discs = std::collections::HashSet::new();
    for t in tracks {
        let val = match t.get("discnumber") {
            Some(Value::Number(n)) => n.as_u64().and_then(|i| u32::try_from(i).ok()).unwrap_or(0),
            Some(Value::String(s)) => s
                .split('/')
                .next()
                .unwrap_or("0")
                .parse::<u32>()
                .unwrap_or(0),
            _ => 0,
        };
        if val > 0 {
            discs.insert(val);
        }
    }
    if discs.is_empty() {
        1
    } else {
        u32::try_from(discs.len()).unwrap_or(u32::MAX)
    }
}

#[must_use]
pub fn resolve_string_fallback(
    source: &Value,
    album_source: &Value,
    key: &str,
    album_key: &str,
    default: &str,
) -> Value {
    let check = |v: &Value| -> Option<String> {
        match v {
            Value::String(s) if !s.trim().is_empty() => Some(s.trim().to_string()),
            Value::Array(a) if !a.is_empty() => Some(a.iter().filter_map(|x| x.as_str()).collect::<Vec<_>>().join("; ")),
            Value::Null => None,
            _ => Some(v.to_string().replace('"', "").trim().to_string()),
        }
    };

    source.get(key).and_then(check).map_or_else(
        || album_source.get(album_key).and_then(check).map_or_else(|| json!(default), |v| json!(v)),
        |v| json!(v)
    )
}

#[must_use]
pub fn format_ms(ms: u64) -> String {
    let s = (ms / 1000) % 60;
    let m = (ms / (1000 * 60)) % 60;
    let h = ms / (1000 * 60 * 60);
    if h > 0 {
        format!("{h}:{m:02}:{s:02}")
    } else {
        format!("{m}:{s:02}")
    }
}

#[must_use]
pub fn rel_path(target: &Path, base: &Path) -> String {
    target.strip_prefix(base).map_or_else(
        |_| target.to_string_lossy().to_string(),
        |p| p.to_string_lossy().to_string(),
    )
}
