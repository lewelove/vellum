use anyhow::{Context, Result};
use lava_torrent::torrent::v1::Torrent;
use std::path::{Path, PathBuf};
use std::process::Command;
use libvellum::config::AppConfig;
use libvellum::utils::expand_path;

#[derive(Debug)]
pub struct AlbumInfo {
    pub pname: String,
    pub source_disk_path: String,
    pub source_disk_hash: String,
    pub torrent_file: String,
    pub torrent_name: String,
    pub torrent_hash: String,
}

pub fn sanitize_source_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

pub fn get_nix32_truncate(hash: &str) -> String {
    if hash.is_empty() {
        return "nohash".to_string();
    }
    
    let output = Command::new("nix")
        .args(["hash", "to-base32", hash])
        .output();
        
    if let Ok(out) = output && out.status.success() {
        let nix32 = String::from_utf8(out.stdout).unwrap_or_default().trim().to_string();
        return nix32.chars().take(32).collect();
    }
    
    hash.trim_start_matches("sha256-").chars().take(32).collect::<String>().replace('/', "_").replace('+', "-")
}

fn eval_nix_field(path: &Path, field_path: &str) -> Result<String> {
    let path_str = path.to_string_lossy();
    let expr = format!(
        "let res = (import (/. + \"{path_str}\") {{ vellum = {{ mkAlbum = x: x; }}; }}); in builtins.toString (res.{field_path} or \"\")"
    );

    let output = Command::new("nix")
        .args(["eval", "--raw", "--impure", "--expr", &expr])
        .output()
        .context("Failed to execute nix eval")?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        anyhow::bail!("Nix evaluation failed for field {field_path}: {err}");
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

pub fn resolve_source_disk(album_info: &AlbumInfo, base_dir: &Path, config: &AppConfig) -> PathBuf {
    if !album_info.source_disk_path.is_empty() && album_info.source_disk_path != "." {
        let p = &album_info.source_disk_path;
        if p.starts_with("./") {
            base_dir.join(p.trim_start_matches("./"))
        } else if p.starts_with('/') {
            PathBuf::from(p)
        } else {
            base_dir.join(p)
        }
    } else {
        let store_root = config.nix.as_ref().map_or_else(
            || base_dir.join(".vellum/store"),
            |n| libvellum::utils::expand_path(&n.store),
        );

        let truncated = get_nix32_truncate(&album_info.torrent_hash);
        let sanitized_source = sanitize_source_name(&album_info.torrent_name);
        let link_name = format!("{sanitized_source}-{truncated}");
        
        let gc_root = store_root.join("gcroots").join("source").join(&link_name);
        if gc_root.exists() {
            return gc_root;
        }

        let stage_root = config.nix.as_ref()
            .and_then(|n| n.stage.clone())
            .map_or_else(|| base_dir.join(".vellum/stage"), |s| libvellum::utils::expand_path(&s));
        
        stage_root.join(link_name)
    }
}

pub fn resolve_template(template: &str, vars: &std::collections::HashMap<String, String>) -> String {
    let re = regex::Regex::new(r"\$\{([^\}]+)\}").unwrap();
    
    let mut result = template.to_string();
    for caps in re.captures_iter(template) {
        let key = &caps[1];
        if let Some(val) = vars.get(key) {
            result = result.replace(&caps[0], val);
        }
    }
    result
}

pub fn parse_album_nix(path: &Path) -> Result<AlbumInfo> {
    let abs_path = path.canonicalize().context("Failed to canonicalize album.nix path")?;
    let album_dir = abs_path.parent().unwrap();
    
    let pname = eval_nix_field(&abs_path, "pname")?;
    let source_disk_path = eval_nix_field(&abs_path, "sourceDisk.path")?;
    let source_disk_hash = eval_nix_field(&abs_path, "sourceDisk.hash")?;
    let torrent_file = eval_nix_field(&abs_path, "sourceTorrent.file")?;
    let mut torrent_name = eval_nix_field(&abs_path, "sourceTorrent.name")?;
    let torrent_hash = eval_nix_field(&abs_path, "sourceTorrent.hash")?;

    if torrent_name.is_empty() && !torrent_file.is_empty() {
        let torrent_path = if torrent_file.starts_with("./") {
            album_dir.join(torrent_file.trim_start_matches("./"))
        } else if torrent_file.starts_with('/') {
            PathBuf::from(&torrent_file)
        } else {
            album_dir.join(&torrent_file)
        };

        if let Ok(t) = Torrent::read_from_file(&torrent_path) {
            torrent_name = t.name;
        }
    }

    Ok(AlbumInfo { 
        pname: if pname.is_empty() { "unknown".to_string() } else { pname },
        source_disk_path, 
        source_disk_hash, 
        torrent_file, 
        torrent_name: if torrent_name.is_empty() { "unknown-source".to_string() } else { torrent_name },
        torrent_hash 
    })
}

pub fn run(album_path: Option<String>) -> Result<()> {
    let (config, _, _) = AppConfig::load().context("Failed to load config")?;
    let nix_config = config.nix.as_ref().context("Missing [nix] configuration")?;
    let cmds = nix_config.commands.as_ref().context("Missing [nix.commands] configuration")?;

    let target_path = if let Some(a) = album_path {
        let p = expand_path(&a)
            .canonicalize()
            .context("Album path does not exist")?;
        if p.is_dir() && p.join("album.nix").exists() {
            p.join("album.nix")
        } else if p.is_file() && p.file_name().unwrap_or_default() == "album.nix" {
            p
        } else {
            anyhow::bail!("No album.nix found at specified path");
        }
    } else {
        let curr = std::env::current_dir()?;
        if curr.join("album.nix").exists() {
            curr.join("album.nix")
        } else {
            anyhow::bail!("No album.nix found in current directory");
        }
    };

    let album_info = parse_album_nix(&target_path)?;
    let album_dir = target_path.parent().unwrap();

    let torrent_file_path = if album_info.torrent_file.starts_with("./") {
        album_dir.join(album_info.torrent_file.trim_start_matches("./"))
    } else if album_info.torrent_file.starts_with('/') {
        PathBuf::from(&album_info.torrent_file)
    } else {
        album_dir.join(&album_info.torrent_file)
    };

    if !torrent_file_path.exists() {
        anyhow::bail!("Torrent file not found: {}", torrent_file_path.display());
    }

    let output = Command::new("nix")
        .args(["hash", "file", torrent_file_path.to_str().unwrap()])
        .output()?;
    let current_sha256 = String::from_utf8(output.stdout)?.trim().to_string();

    if current_sha256 != album_info.torrent_hash {
        anyhow::bail!("Torrent file hash mismatch! Expected {}, got {}", album_info.torrent_hash, current_sha256);
    }
    
    let source_disk = resolve_source_disk(&album_info, album_dir, &config);

    let mut vars = std::collections::HashMap::new();
    vars.insert("sourceDisk.path".to_string(), source_disk.to_string_lossy().to_string());
    vars.insert("sourceTorrent.file".to_string(), torrent_file_path.to_string_lossy().to_string());
    vars.insert("sourceTorrent.hash".to_string(), album_info.torrent_hash);
    vars.insert("sourceTorrent.name".to_string(), album_info.torrent_name);

    let cmd_tpl = cmds.get("get_torrent").context("No 'get_torrent' command configured")?;
    let final_cmd = resolve_template(cmd_tpl, &vars);

    log::info!("Executing: {final_cmd}");
    let status = std::process::Command::new("sh").arg("-c").arg(&final_cmd).status()?;

    if status.success() {
        log::info!("Download initiated to {}", source_disk.display());
    } else {
        log::error!("Command failed with status: {status}");
    }

    Ok(())
}
