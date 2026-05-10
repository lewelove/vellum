pub mod globbing;

use anyhow::Result;
use lava_torrent::torrent::v1::Torrent;
use std::fmt::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml::Value;

pub fn run(torrent_path_str: &str, tracks_filter: &str, metadata_paths: Option<String>) -> Result<()> {
    let torrent_path = Path::new(torrent_path_str).canonicalize()?;
    let current_dir = std::env::current_dir()?;

    let output = Command::new("nix")
        .args(["hash", "file", torrent_path.to_str().unwrap()])
        .output()?;

    if !output.status.success() {
        anyhow::bail!("Failed to hash torrent file with nix");
    }

    let torrent_hash = String::from_utf8(output.stdout)?.trim().to_string();
    let torrent = Torrent::read_from_file(&torrent_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse torrent: {e}"))?;

    let globset = globbing::build_globset(tracks_filter)?;
    let mut valid_paths = Vec::new();

    if let Some(files) = &torrent.files {
        for f in files {
            let path_buf = f.path.clone();
            if globset.is_match(path_buf.to_string_lossy().as_ref()) {
                valid_paths.push(path_buf);
            }
        }
    } else {
        let name_str = &torrent.name;
        if globset.is_match(name_str) {
            valid_paths.push(Path::new(name_str).to_path_buf());
        }
    }

    valid_paths.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));

    let rel_torrent = torrent_path
        .strip_prefix(&current_dir)
        .map_or_else(|_| torrent_path.clone(), Path::to_path_buf);

    let torrent_nix_path = if rel_torrent.is_absolute() {
        format!("\"{}\"", rel_torrent.to_string_lossy())
    } else {
        format!("./{}", rel_torrent.to_string_lossy())
    };

    let effective_paths = metadata_paths.or_else(|| {
        let default = Path::new("metadata.toml");
        if default.exists() {
            Some("metadata.toml".to_string())
        } else {
            None
        }
    });

    let merged_meta = merge_metadata_files(effective_paths)?;
    let output_nix = generate_nix_content(
        &torrent,
        &valid_paths,
        &torrent_hash,
        &torrent_nix_path,
        &merged_meta,
    );

    println!("{output_nix}");
    Ok(())
}

fn merge_metadata_files(metadata_paths: Option<String>) -> Result<Value> {
    let mut merged = Value::Table(toml::map::Map::new());

    if let Some(paths) = metadata_paths {
        let path_list: Vec<&str> = paths.split(',').map(str::trim).collect();
        for p_str in path_list.iter().rev() {
            let p = Path::new(p_str);
            if p.exists() {
                let content = std::fs::read_to_string(p)?;
                let parsed: Value = toml::from_str(&content)?;
                deep_merge(&mut merged, parsed);
            }
        }
    }
    Ok(merged)
}

fn generate_nix_content(
    torrent: &Torrent,
    valid_paths: &[PathBuf],
    torrent_hash: &str,
    torrent_nix_path: &str,
    merged_meta: &Value,
) -> String {
    let album_data = merged_meta.get("album");
    let artist = album_data
        .and_then(|a| a.get("albumartist"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let album = album_data
        .and_then(|a| a.get("album"))
        .and_then(Value::as_str)
        .unwrap_or(&torrent.name);

    let pname_base = if artist.is_empty() {
        album.to_lowercase()
    } else {
        format!("{}-{}", artist.to_lowercase(), album.to_lowercase())
    };

    let sanitized_pname = pname_base
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-");

    let mut out = String::new();
    let _ = writeln!(out, "{{ vellum }}:\n");
    let _ = writeln!(out, "vellum.mkAlbum {{\n");
    let _ = writeln!(out, "  pname = \"{sanitized_pname}\";\n");
    let _ = writeln!(out, "  sourceTorrent = {{");
    let _ = writeln!(out, "    file = {torrent_nix_path};");
    let _ = writeln!(out, "    hash = \"{torrent_hash}\";");
    let _ = writeln!(out, "  }};\n");
    let _ = writeln!(
        out,
        "  sourceDisk.hash = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\";\n"
    );

    let _ = writeln!(out, "  album = {{");
    let _ = writeln!(out, "    metadata = {{");
    if let Some(data) = album_data {
        let _ = write!(out, "{}", to_nix_attributes(data, "      "));
    }
    let _ = writeln!(out, "    }};\n  }};\n");

    let _ = writeln!(out, "  cover = ./cover.png;\n");
    let _ = writeln!(out, "  tracks = [");

    let meta_tracks = merged_meta.get("tracks").and_then(Value::as_array);
    let meta_tracks_len = meta_tracks.map_or(0, Vec::len);
    let total_count = std::cmp::max(valid_paths.len(), meta_tracks_len);

    for i in 0..total_count {
        let file_path = valid_paths.get(i).map_or_else(String::new, |path_buf| {
            let inner_path_str = path_buf.to_string_lossy();
            if torrent.files.is_some() {
                format!("{}/{}", torrent.name, inner_path_str)
            } else {
                inner_path_str.to_string()
            }
        });

        let track_meta = if let Some(arr) = meta_tracks
            && i < arr.len()
        {
            arr[i].clone()
        } else {
            Value::Table(toml::map::Map::new())
        };

        let _ = writeln!(out, "    {{");
        let _ = writeln!(out, "      file = \"{file_path}\";");
        let _ = writeln!(out, "      metadata = {{");
        let _ = write!(out, "{}", to_nix_attributes(&track_meta, "        "));
        let _ = writeln!(out, "      }};");
        let _ = write!(out, "    }}");
        if i < total_count - 1 {
            let _ = writeln!(out);
        }
    }

    let _ = writeln!(out, "\n  ];\n}}");
    out
}

fn to_nix_attributes(val: &Value, indent: &str) -> String {
    let mut res = String::new();
    if let Some(tab) = val.as_table() {
        for (k, v) in tab {
            let _ = writeln!(res, "{indent}{k} = {};", to_nix_value(v));
        }
    }
    res
}

fn to_nix_value(val: &Value) -> String {
    match val {
        Value::String(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Value::Integer(i) => i.to_string(),
        Value::Float(f) => f.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Array(arr) => {
            let mut items = vec![];
            for v in arr {
                items.push(to_nix_value(v));
            }
            format!("[ {} ]", items.join(" "))
        }
        _ => "\"\"".to_string(),
    }
}

fn deep_merge(base: &mut Value, overlay: Value) {
    match (base, overlay) {
        (Value::Table(base_map), Value::Table(overlay_map)) => {
            for (k, v) in overlay_map {
                if k == "tracks" && v.is_array() {
                    if let Some(base_tracks) = base_map.get_mut("tracks") {
                        if let (Value::Array(b_arr), Value::Array(o_arr)) = (base_tracks, v) {
                            for (i, o_val) in o_arr.into_iter().enumerate() {
                                if i < b_arr.len() {
                                    deep_merge(&mut b_arr[i], o_val);
                                } else {
                                    b_arr.push(o_val);
                                }
                            }
                        }
                    } else {
                        base_map.insert(k, v);
                    }
                } else if let Some(base_v) = base_map.get_mut(&k) {
                    deep_merge(base_v, v);
                } else {
                    base_map.insert(k, v);
                }
            }
        }
        (base_val, overlay_val) => {
            *base_val = overlay_val;
        }
    }
}
