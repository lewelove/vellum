use anyhow::{Context, Result};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::process::Command;
use libvellum::config::AppConfig;
use libvellum::utils::expand_path;

#[derive(Clone, Copy)]
struct PinOptions<'a> {
    target: &'a Path,
    store_path: &'a Path,
    staging_src_env: &'a str,
    torrent_name: &'a str,
    flake_uri: &'a str,
    config: &'a AppConfig,
    link_name: &'a str,
}

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
    
    sync_env(store_path, flake_uri, target)?;
    
    let album_info = crate::nix::get::parse_album_nix(target_path)?;
    let source_disk = crate::nix::get::resolve_source_disk(&album_info, target, config);

    let physical_source_disk = source_disk
        .canonicalize()
        .unwrap_or_else(|_| source_disk.clone());

    let staging_src_env = physical_source_disk.to_string_lossy().to_string();
    let is_source_in_store = physical_source_disk.starts_with(store_path);

    if is_source_in_store {
        log::info!("Found sourceDisk.path in store. Skipping verification...");
    } else {
        verify_hash(&physical_source_disk, &album_info.source_disk_hash, "Source disk")?;
    }

    if album_info.cover_file.starts_with('/') {
        let cover_path = std::path::PathBuf::from(&album_info.cover_file);
        if cover_path.starts_with(store_path) {
            log::info!("Found cover.file in store. Skipping verification...");
        } else {
            verify_hash(&cover_path, &album_info.cover_hash, "Cover image")?;
        }
    }

    let torrent_file_path = if album_info.torrent_file.starts_with("./") {
        target.join(album_info.torrent_file.trim_start_matches("./"))
    } else if album_info.torrent_file.starts_with('/') {
        std::path::PathBuf::from(&album_info.torrent_file)
    } else {
        target.join(&album_info.torrent_file)
    };

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
    vars.insert("sourceTorrent.name".to_string(), album_info.torrent_name.clone());

    if is_source_in_store {
    } else {
        run_verification(config, &vars, target)?;
    }

    let truncated_hash = crate::nix::get::get_nix32_truncate(&album_info.torrent_hash);
    let sanitized_source = crate::nix::get::sanitize_source_name(&album_info.torrent_name);
    let link_name = format!("{sanitized_source}-{truncated_hash}");

    let gc_roots_albums = store_path.join("gcroots").join("albums");
    fs::create_dir_all(&gc_roots_albums).context("Failed to create gcroots/albums directory")?;

    let result_link = gc_roots_albums.join(format!("{}-{}", album_info.pname, truncated_hash));
    execute_nix_build(target, store_path, &staging_src_env, &sanitized_source, flake_uri, &result_link)?;

    let logical_path = fs::read_link(&result_link).with_context(|| {
        format!(
            "Nix build claimed success but result link {} was not found",
            result_link.display()
        )
    })?;

    let physical_store_path =
        store_path.join(logical_path.strip_prefix("/").unwrap_or(&logical_path));
        
    materialize_output(&physical_store_path, target, store_path)?;

    let pin_opts = PinOptions {
        target,
        store_path,
        staging_src_env: &staging_src_env,
        torrent_name: &sanitized_source,
        flake_uri,
        config,
        link_name: &link_name,
    };

    pin_source_and_seed(pin_opts, &mut vars)?;

    Ok(())
}

fn sync_env(store_path: &Path, flake_uri: &str, target_dir: &Path) -> Result<()> {
    let gc_roots_profiles = store_path.join("gcroots").join("profiles");
    fs::create_dir_all(&gc_roots_profiles).context("Failed to create gcroots/profiles directory")?;
    let active_env_link = gc_roots_profiles.join("env");

    let mut cmd = Command::new("nix");
    cmd.arg("build")
        .arg("--store")
        .arg(store_path)
        .arg("--impure")
        .arg(format!("{flake_uri}#env"))
        .arg("--out-link")
        .arg(&active_env_link)
        .current_dir(target_dir);

    let status = cmd.status().context("Failed to execute nix build for env")?;
    if !status.success() {
        anyhow::bail!(
            "Nix build failed with exit code {} for env",
            status.code().unwrap_or(-1)
        );
    }
    Ok(())
}

fn verify_hash(target_path: &Path, expected_hash: &str, name: &str) -> Result<()> {
    if !target_path.exists() {
        anyhow::bail!("{name} path does not exist at: {}", target_path.display());
    }

    let mode = if target_path.is_dir() { "path" } else { "file" };

    let hash_output = Command::new("nix")
        .args(["hash", mode, target_path.to_str().unwrap()])
        .output()
        .context(format!("Failed to compute {name} hash using nix"))?;

    let actual_hash = String::from_utf8_lossy(&hash_output.stdout).trim().to_string();

    if expected_hash.is_empty() || expected_hash != actual_hash {
        anyhow::bail!(
            "{name} hash mismatch or missing in manifest.\n\nExpected: '{}'\nActual:   '{}'\n\nPlease update the hash in album.nix to the actual value.",
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
    torrent_name: &str,
    flake_uri: &str,
    result_link: &Path,
) -> Result<()> {
    let expr =
        format!("(import ./album.nix {{ vellum = (builtins.getFlake \"{flake_uri}\").lib; }})");
    let mut cmd = Command::new("nix");
    cmd.env("VELLUM_STAGING_SRC", staging_src_env);
    cmd.env("VELLUM_TORRENT_NAME", torrent_name);
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
    opts: PinOptions,
    vars: &mut HashMap<String, String>,
) -> Result<()> {
    let expr_source = format!(
        "(import ./album.nix {{ vellum = (builtins.getFlake \"{}\").lib; }}).sourceStorePath",
        opts.flake_uri
    );
    
    let eval_output = Command::new("nix")
        .env("VELLUM_STAGING_SRC", opts.staging_src_env)
        .env("VELLUM_TORRENT_NAME", opts.torrent_name)
        .arg("eval")
        .arg("--store")
        .arg(opts.store_path)
        .arg("--raw")
        .arg("--impure")
        .arg("--expr")
        .arg(&expr_source)
        .current_dir(opts.target)
        .output()?;

    let source_store_path_raw = String::from_utf8_lossy(&eval_output.stdout).trim().to_string();
    if source_store_path_raw.is_empty() || !source_store_path_raw.starts_with("/nix/store") {
        return Ok(());
    }

    let mapped_source_store_path = source_store_path_raw.strip_prefix("/").map_or_else(
        || source_store_path_raw.clone(),
        |stripped| opts.store_path.join(stripped).to_string_lossy().to_string(),
    );

    let gc_roots_source = opts.store_path.join("gcroots").join("source");
    fs::create_dir_all(&gc_roots_source)?;
    let source_link = gc_roots_source.join(opts.link_name);

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

    if let Some(nix_cfg) = &opts.config.nix
        && let Some(cmds) = &nix_cfg.commands
        && let Some(seed_cmd_tpl) = cmds.get("seed_torrent")
    {
        let final_seed_cmd = crate::nix::get::resolve_template(seed_cmd_tpl, vars);
        log::info!("Executing seed command: {final_seed_cmd}");
        let _ = Command::new("sh")
            .arg("-c")
            .arg(&final_seed_cmd)
            .current_dir(opts.target)
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
