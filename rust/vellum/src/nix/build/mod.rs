use anyhow::{Context, Result};
use lava_torrent::torrent::v1::Torrent;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use libvellum::config::AppConfig;
use libvellum::utils::expand_path;

pub fn run(album_path: Option<String>) -> Result<()> {
    let (config, _, _) = AppConfig::load().context("Failed to load config")?;
    let nix_config = config.nix.as_ref().context("Missing [nix] configuration in config.toml")?;
    let store_path = expand_path(&nix_config.store)
        .canonicalize()
        .context("Custom nix store path does not exist or is inaccessible")?;

    let mut flake_uri = nix_config.flake.clone();
    if flake_uri.starts_with('/') || flake_uri.starts_with('~') {
        let expanded = expand_path(&flake_uri);
        let canon = expanded.canonicalize().context("Could not find flake path")?;
        
        let flake_dir = if canon.is_file() {
            canon.parent().context("Flake path has no parent")?.to_path_buf()
        } else {
            canon
        };
        
        flake_uri = format!("path:{}", flake_dir.display());
    }

    let target_path = if let Some(a) = album_path {
        let p = expand_path(&a).canonicalize().context("Album path does not exist")?;
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

    build_album(&target_path, &store_path, &flake_uri, &config)?;

    Ok(())
}

fn build_album(
    target_path: &Path,
    store_path: &Path,
    flake_uri: &str,
    config: &AppConfig,
) -> Result<()> {
    let target = target_path.parent().unwrap();
    let album_info = crate::nix::get::parse_album_nix(target_path)?;
    let source_disk = crate::nix::get::resolve_source_disk(&album_info, target, config);

    let physical_source_disk = source_disk
        .canonicalize()
        .unwrap_or_else(|_| source_disk.clone());

    let staging_src_env = physical_source_disk.to_string_lossy().to_string();

    verify_source_hash(&physical_source_disk, &album_info.source_disk_hash)?;

    let torrent_file_path = if album_info.torrent_file.starts_with("./") {
        target.join(album_info.torrent_file.trim_start_matches("./"))
    } else {
        target.join(&album_info.torrent_file)
    };

    let torrent_name = Torrent::read_from_file(&torrent_file_path)
        .map_or_else(|_| album_info.pname.clone(), |t| t.name);

    let mut vars = HashMap::new();
    vars.insert(
        "sourceDisk.path".to_string(),
        physical_source_disk.to_string_lossy().to_string(),
    );
    vars.insert(
        "sourceTorrent.file".to_string(),
        torrent_file_path.to_string_lossy().to_string(),
    );
    vars.insert(
        "sourceTorrent.hash".to_string(),
        album_info.torrent_hash.clone(),
    );
    vars.insert("sourceTorrent.name".to_string(), torrent_name);

    run_verification(config, &vars, target)?;

    let truncated_hash = crate::nix::get::get_nix32_truncate(&album_info.torrent_hash);
    let sanitized_pname = album_info.pname.replace('"', "").trim().replace(' ', "-");
    let link_name = format!("{sanitized_pname}-{truncated_hash}");

    let gc_roots_albums = store_path.join("gcroots").join("albums");
    fs::create_dir_all(&gc_roots_albums).context("Failed to create gcroots/albums directory")?;

    let result_link = gc_roots_albums.join(&link_name);
    execute_nix_build(target, store_path, &staging_src_env, flake_uri, &result_link)?;

    let logical_path = fs::read_link(&result_link).with_context(|| {
        format!(
            "Nix build claimed success but result link {} was not found",
            result_link.display()
        )
    })?;

    let physical_store_path =
        store_path.join(logical_path.strip_prefix("/").unwrap_or(&logical_path));
        
    materialize_output(&physical_store_path, target, store_path)?;

    pin_source_and_seed(
        target,
        store_path,
        &staging_src_env,
        flake_uri,
        config,
        &link_name,
        &mut vars,
    )?;

    Ok(())
}

fn verify_source_hash(source_disk: &Path, expected_hash: &str) -> Result<()> {
    if !source_disk.exists() {
        anyhow::bail!("Source path does not exist at: {}", source_disk.display());
    }

    let hash_output = Command::new("nix")
        .args(["hash", "path", source_disk.to_str().unwrap()])
        .output()
        .context("Failed to compute source hash using nix")?;

    let actual_hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();

    if expected_hash.is_empty() || expected_hash != actual_hash {
        anyhow::bail!(
            "Source hash mismatch or missing in manifest.\n\nExpected: '{}'\nActual:   '{}'\n\nPlease update `sourceDisk.hash` in album.nix to the actual value.",
            if expected_hash.is_empty() { "none" } else { expected_hash },
            actual_hash
        );
    }

    Ok(())
}

fn run_verification(config: &AppConfig, vars: &HashMap<String, String>, target: &Path) -> Result<()> {
    if let Some(nix_cfg) = &config.nix
        && let Some(cmds) = &nix_cfg.commands
        && let Some(verify_cmd_tpl) = cmds.get("verify_torrent")
    {
        let final_cmd = crate::nix::get::resolve_template(verify_cmd_tpl, vars);
        log::info!("Executing verification: {final_cmd}");

        let status = Command::new("sh")
            .arg("-c")
            .arg(&final_cmd)
            .current_dir(target)
            .status()?;

        if !status.success() {
            anyhow::bail!("Verification command failed with status: {status}");
        }
        log::info!("Seeding check passed!");
    }
    Ok(())
}

fn execute_nix_build(
    target: &Path,
    store_path: &Path,
    staging_src_env: &str,
    flake_uri: &str,
    result_link: &Path,
) -> Result<()> {
    let expr =
        format!("(import ./album.nix {{ vellum = (builtins.getFlake \"{flake_uri}\").lib; }})");
    let mut cmd = Command::new("nix");
    cmd.env("VELLUM_STAGING_SRC", staging_src_env);
    cmd.arg("build")
        .arg("--store")
        .arg(store_path)
        .arg("--impure")
        .arg("--expr")
        .arg(&expr)
        .arg("--out-link")
        .arg(result_link)
        .current_dir(target);

    let status = cmd.status().context("Failed to execute nix build binary")?;
    if !status.success() {
        anyhow::bail!(
            "Nix build failed with exit code {} for {}",
            status.code().unwrap_or(-1),
            target.display()
        );
    }
    Ok(())
}

fn pin_source_and_seed(
    target: &Path,
    store_path: &Path,
    staging_src_env: &str,
    flake_uri: &str,
    config: &AppConfig,
    link_name: &str,
    vars: &mut HashMap<String, String>,
) -> Result<()> {
    let expr_source = format!(
        "(import ./album.nix {{ vellum = (builtins.getFlake \"{flake_uri}\").lib; }}).sourceStorePath"
    );
    
    let eval_output = Command::new("nix")
        .env("VELLUM_STAGING_SRC", staging_src_env)
        .arg("eval")
        .arg("--store")
        .arg(store_path)
        .arg("--raw")
        .arg("--impure")
        .arg("--expr")
        .arg(&expr_source)
        .current_dir(target)
        .output()?;

    let source_store_path_raw = String::from_utf8_lossy(&eval_output.stdout).trim().to_string();
    if source_store_path_raw.is_empty() || !source_store_path_raw.starts_with("/nix/store") {
        return Ok(());
    }

    let mapped_source_store_path = source_store_path_raw.strip_prefix("/").map_or_else(
        || source_store_path_raw.clone(),
        |stripped| store_path.join(stripped).to_string_lossy().to_string(),
    );

    let gc_roots_source = store_path.join("gcroots").join("source");
    fs::create_dir_all(&gc_roots_source)?;
    let source_link = gc_roots_source.join(link_name);

    if source_link.exists() || source_link.symlink_metadata().is_ok() {
        fs::remove_file(&source_link)?;
    }

    if let Err(e) = std::os::unix::fs::symlink(&mapped_source_store_path, &source_link) {
        log::warn!(
            "Failed to create source GC root link {}: {}",
            source_link.display(),
            e
        );
    }

    vars.insert("sourceStorePath".to_string(), mapped_source_store_path);

    if let Some(nix_cfg) = &config.nix
        && let Some(cmds) = &nix_cfg.commands
        && let Some(seed_cmd_tpl) = cmds.get("seed_torrent")
    {
        let final_seed_cmd = crate::nix::get::resolve_template(seed_cmd_tpl, vars);
        log::info!("Executing seed command: {final_seed_cmd}");
        let _ = Command::new("sh")
            .arg("-c")
            .arg(&final_seed_cmd)
            .current_dir(target)
            .status();
    }

    Ok(())
}

fn materialize_output(store_dir: &Path, target_dir: &Path, store_path: &Path) -> Result<()> {
    let entries = fs::read_dir(store_dir).with_context(|| {
        format!(
            "Could not read nix store directory: {}",
            store_dir.display()
        )
    })?;

    for entry in entries {
        let entry = entry?;
        let file_name = entry.file_name();
        let file_type = entry.file_type()?;

        if file_name == "album.nix" {
            continue;
        }

        let mut store_file = entry.path();
        let target_file = target_dir.join(&file_name);

        if let Ok(meta) = fs::symlink_metadata(&target_file) {
            if meta.is_dir() {
                fs::remove_dir_all(&target_file)?;
            } else {
                fs::remove_file(&target_file)?;
            }
        }

        if file_type.is_symlink() {
            let resolved_path = fs::read_link(&store_file)?;
            if resolved_path.starts_with("/nix/store") {
                let stripped = resolved_path.strip_prefix("/").unwrap();
                store_file = store_path.join(stripped);
            } else if resolved_path.is_relative() {
                store_file = store_dir.join(resolved_path);
            } else {
                store_file = resolved_path;
            }
        }

        if fs::hard_link(&store_file, &target_file).is_err() {
            std::os::unix::fs::symlink(&store_file, &target_file).with_context(|| {
                format!(
                    "Failed to create link (hard or sym) for {} at {}",
                    store_file.display(),
                    target_file.display()
                )
            })?;
        }
    }
    Ok(())
}
