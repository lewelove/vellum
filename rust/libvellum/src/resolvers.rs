use std::path::Path;

#[must_use]
pub fn calculate_total_discs(tracks: &[serde_json::Value]) -> u32 {
    let mut discs = std::collections::HashSet::new();
    for t in tracks {
        let val = match t.get("discnumber") {
            Some(serde_json::Value::Number(n)) => n.as_u64().and_then(|i| u32::try_from(i).ok()).unwrap_or(0),
            Some(serde_json::Value::String(s)) => s
                .split('/')
                .next()
                .unwrap_or("0")
                .trim()
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

pub fn resolve_album_info_date_added(album_root: &Path, source: &serde_json::Value, config: &crate::lua::ResolvedConfig) -> Result<String, crate::error::VellumError> {
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
        let json_val = serde_json::Value::String(dt_str);
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
    let folders = ["lyrics", "Lyrics"];
    let mut candidates = Vec::new();

    for folder in folders {
        let dir = album_root.join(folder);
        let Ok(entries) = std::fs::read_dir(dir) else { continue; };

        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if !path.is_file() { continue; }
            let Some(ext) = path.extension().and_then(|e| e.to_str()).map(str::to_lowercase) else { continue; };
            if ext != "lrc" && ext != "txt" { continue; }

            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue; };

            let is_match = if total_discs > 1 {
                name.find('.').is_some_and(|dot_idx| {
                    let disc_part = &name[..dot_idx];
                    let remaining = &name[dot_idx + 1..];
                    let track_part = remaining.chars().take_while(char::is_ascii_digit).collect::<String>();
                    let d_match = disc_part.parse::<u32>().is_ok_and(|d| d == disc_num);
                    let t_match = track_part.parse::<u32>().is_ok_and(|t| t == track_num);
                    d_match && t_match
                })
            } else {
                let track_part = name.chars().take_while(char::is_ascii_digit).collect::<String>();
                track_part.parse::<u32>().is_ok_and(|t| t == track_num)
            };

            if is_match {
                candidates.push((rel_path(&path, album_root), ext));
            }
        }
    }

    if candidates.is_empty() { return None; }
    if let Some(lrc) = candidates.iter().find(|(_, ext)| ext == "lrc") {
        return Some(lrc.0.clone());
    }
    candidates.first().map(|(path, _)| path.clone())
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
