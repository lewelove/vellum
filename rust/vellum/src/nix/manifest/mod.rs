use anyhow::Result;
use lava_torrent::torrent::v1::Torrent;
use std::fmt::Write;
use std::path::Path;

pub fn run(torrent_path_str: &str, tracks_filter: &str) -> Result<()> {
    let torrent_path = Path::new(torrent_path_str).canonicalize()?;
    let current_dir = std::env::current_dir()?;
    
    let output = std::process::Command::new("nix")
        .args(["hash", "file", torrent_path.to_str().unwrap()])
        .output()?;
    
    if !output.status.success() {
        anyhow::bail!("Failed to hash torrent file with nix");
    }
    
    let torrent_hash = String::from_utf8(output.stdout)?.trim().to_string();
    let torrent = Torrent::read_from_file(&torrent_path)
        .map_err(|e| anyhow::anyhow!("Failed to parse torrent: {e}"))?;
        
    let allowed_exts: Vec<String> = tracks_filter
        .split(',')
        .map(|s| format!(".{}", s.trim().to_lowercase()))
        .collect();
        
    let mut valid_paths = Vec::new();
    
    if let Some(files) = &torrent.files {
        for f in files {
            let path_buf = f.path.clone();
            let ext = path_buf.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
            if allowed_exts.contains(&ext) {
                valid_paths.push(path_buf);
            }
        }
    } else {
        let name_str = &torrent.name;
        let path = Path::new(name_str);
        let ext = path.extension().and_then(|e| e.to_str()).map(|e| format!(".{}", e.to_lowercase())).unwrap_or_default();
        if allowed_exts.contains(&ext) {
            valid_paths.push(path.to_path_buf());
        }
    }

    valid_paths.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));
    
    let mut track_lines = Vec::new();
    let mut track_no = 1;
    
    if torrent.files.is_some() {
        for path_buf in valid_paths {
            let inner_path_str = path_buf.to_string_lossy();
            let file_path = format!("{}/{}", torrent.name, inner_path_str);
            let title = path_buf.file_stem().unwrap_or_default().to_string_lossy().replace('"', "\\\"");
            
            track_lines.push(format!(
                "    {{\n      file = \"{file_path}\";\n      metadata = {{\n        tracknumber = {track_no};\n        title = \"{title}\";\n      }};\n    }}"
            ));
            track_no += 1;
        }
    } else {
        for path_buf in valid_paths {
            let file_path = path_buf.to_string_lossy();
            let title = path_buf.file_stem().unwrap_or_default().to_string_lossy().replace('"', "\\\"");
            
            track_lines.push(format!(
                "    {{\n      file = \"{file_path}\";\n      metadata = {{\n        tracknumber = {track_no};\n        title = \"{title}\";\n      }};\n    }}"
            ));
            track_no += 1;
        }
    }

    let rel_torrent = torrent_path
        .strip_prefix(&current_dir)
        .map_or_else(|_| torrent_path.clone(), Path::to_path_buf);

    let torrent_nix_path = if rel_torrent.is_absolute() {
        rel_torrent.to_string_lossy().into_owned()
    } else {
        format!("./{}", rel_torrent.to_string_lossy())
    };

    let sanitized_pname = torrent.name.replace(' ', "-").replace(['(', ')', '[', ']', '_'], "-").to_lowercase();
    
    let mut out = String::new();
    out.push_str("{ vellum }:\n\n");
    out.push_str("vellum.mkAlbum {\n\n");
    let _ = writeln!(out, "  pname = \"{sanitized_pname}\";\n");
    out.push_str("  sourceTorrent = {\n");
    let _ = writeln!(out, "    file = {torrent_nix_path};");
    let _ = writeln!(out, "    hash = \"{torrent_hash}\";");
    out.push_str("  };\n\n");
    out.push_str("  sourceDisk.hash = \"sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=\";\n\n");
    out.push_str("  album = {\n");
    out.push_str("    metadata = {\n");
    out.push_str("      albumartist = \"\";\n");
    let _ = writeln!(out, "      album = \"{}\";", torrent.name.replace('\"', "\\\""));
    out.push_str("      date = \"\";\n");
    out.push_str("      genre = \"\";\n");
    out.push_str("    };\n");
    out.push_str("  };\n\n");
    out.push_str("  cover = ./cover.png;\n\n");
    out.push_str("  tracks = [\n");
    for (i, line) in track_lines.iter().enumerate() {
        out.push_str(line);
        if i < track_lines.len() - 1 {
            out.push('\n');
        }
    }
    out.push_str("\n  ];\n");
    out.push_str("}\n");
    
    println!("{out}");
    Ok(())
}
