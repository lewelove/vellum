use crate::compile::builder::context::AlbumContext;
use crate::compile::resolvers::standard;
use serde_json::{Value, json};
use std::path::Path;

pub fn calculate_total_discs(tracks: &[Value]) -> u32 {
    let mut discs = std::collections::HashSet::new();
    for t in tracks {
        let val = match t.get("discnumber") {
            Some(Value::Number(n)) => n.as_u64().unwrap_or(0),
            Some(Value::String(s)) => s
                .split('/')
                .next()
                .unwrap_or("0")
                .parse::<u64>()
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

pub fn resolve_album_info_date_added(ctx: &AlbumContext, _args: &str) -> String {
    let library_toml_path = ctx.album_root.join("library.toml");
    if library_toml_path.exists()
        && let Ok(content) = std::fs::read_to_string(&library_toml_path)
        && let Ok(parsed) = toml::from_str::<toml::Value>(&content)
        && let Some(lib) = parsed.get("library")
        && let Some(da) = lib.get("date_added")
    {
        let json_val = serde_json::to_value(da).unwrap_or(Value::Null);
        return standard::parse_time(Some(&json_val));
    }

    let mut keys = Vec::new();
    if let Some(fallbacks) = ctx.config.get("compiler").and_then(|c| c.get("date_added")).and_then(Value::as_array) {
        for f in fallbacks {
            if let Some(s) = f.as_str() {
                keys.push(s.to_string());
            }
        }
    }

    for key in &keys {
        if let Some(val) = ctx.source.get(key) {
            return standard::parse_time(Some(val));
        }
    }

    standard::parse_time(None)
}

pub fn rel_path(target: &Path, base: &Path) -> String {
    target.strip_prefix(base).map_or_else(
        |_| target.to_string_lossy().to_string(),
        |p| p.to_string_lossy().to_string(),
    )
}

pub fn resolve_cover_chroma(ctx: &AlbumContext, _args: &str) -> Option<Value> {
    ctx.cover_metrics.and_then(|m| m.chroma).map(|c| json!(c))
}

pub fn resolve_cover_entropy(ctx: &AlbumContext, _args: &str) -> Option<Value> {
    ctx.cover_metrics.and_then(|m| m.entropy).map(|e| json!(e))
}

pub fn resolve_cover_palette(ctx: &AlbumContext, _cfg: &Value) -> Option<Value> {
    ctx.cover_metrics.and_then(|m| m.palette.clone())
}

pub fn resolve_comment(ctx: &AlbumContext, _args: &str) -> String {
    if let Some(v) = ctx.source.get("comment").and_then(Value::as_str)
        && !v.is_empty()
    {
        return v.to_string();
    }

    let country = standard::resolve_generic_string(ctx.source, "country", "", "").as_str().unwrap_or("").to_string();
    let label = standard::resolve_generic_string(ctx.source, "label", "", "").as_str().unwrap_or("").to_string();
    let cat = standard::resolve_generic_string(ctx.source, "catalognumber", "", "").as_str().unwrap_or("").to_string();
    if country.is_empty() && label.is_empty() && cat.is_empty() {
        return String::new();
    }
    let yyyy_mm = resolve_yyyy_mm(ctx, "release_yyyy_mm", "");
    let year = if yyyy_mm.len() >= 4 {
        &yyyy_mm[0..4]
    } else {
        ""
    };
    
    let parts =[
        year,
        &country,
        &label,
        &cat,
    ];

    parts
        .iter()
        .filter(|s| !s.is_empty())
        .copied()
        .collect::<Vec<_>>()
        .join(" ")
}

pub fn resolve_yyyy_mm(ctx: &AlbumContext, key: &str, _args: &str) -> String {
    if let Some(v) = ctx.source.get(key).and_then(Value::as_str) {
        return v.to_string();
    }
    let d = standard::resolve_generic_string(ctx.source, "date", "year,originalyear", "0000").as_str().unwrap_or("0000").to_string();
    if d.len() >= 4 {
        format!("{}-00", &d[0..4])
    } else {
        "0000-00".to_string()
    }
}

pub fn resolve_lyrics_path(
    album_root: &Path,
    track_num: u32,
    disc_num: u32,
    total_discs: u32,
) -> Option<String> {
    let folders =[
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
