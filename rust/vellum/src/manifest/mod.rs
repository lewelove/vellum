pub mod compressor;
pub mod engine;
pub mod grouper;

use libvellum::utils::expand_path;
use libvellum::harvest::harvest_file;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub enum ManifestMode {
    Album,
    Library,
}

pub struct ManifestOptions {
    pub mode: ManifestMode,
    pub force: bool,
    pub stdout: bool,
}

pub fn run(target_path: Option<PathBuf>, options: &ManifestOptions) -> Result<()> {
    let config = libvellum::lua::ResolvedConfig::load().context("Failed to load config")?;
    let lib_root = expand_path(&config.app.storage.library);

    let manifest_cfg = &config.app.manifest;
    let supported_exts: Vec<String> = manifest_cfg
        .audio_files
        .as_ref().map_or_else(|| vec![".flac".to_string()], |exts: &Vec<String>| exts.iter().map(|e| e.to_lowercase()).collect());

    let grouping_keys = vec!["albumartist".to_string(), "album".to_string()];

    let scan_root = match options.mode {
        ManifestMode::Library => target_path.unwrap_or_else(|| lib_root.clone()),
        ManifestMode::Album => target_path.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))),
    };

    let dirs_to_harvest = match options.mode {
        ManifestMode::Album => vec![scan_root],
        ManifestMode::Library => find_harvestable_directories(&scan_root, options.force, &supported_exts),
    };

    if dirs_to_harvest.is_empty() {
        if !options.stdout {
            log::info!("No new audio directories found.");
        }
        return Ok(());
    }

    let harvested = harvest_audio_files(dirs_to_harvest, &supported_exts, options);
    if harvested.is_empty() {
        return Ok(());
    }

    let buckets = grouper::group_tracks(harvested, &grouping_keys);

    for (_group_id, tracks) in buckets {
        process_album_group(tracks, &supported_exts, options)?;
    }

    Ok(())
}

fn process_album_group(
    mut tracks: Vec<(PathBuf, serde_json::Map<String, serde_json::Value>)>,
    supported_exts: &[String],
    options: &ManifestOptions,
) -> Result<()> {
    let validate_exclusivity = match options.mode {
        ManifestMode::Album => false,
        ManifestMode::Library => true,
    };
    
    let (anchor_opt, is_valid) = grouper::resolve_anchor(&tracks, validate_exclusivity, supported_exts);
    if !is_valid {
        return Ok(());
    }

    let Some(anchor) = anchor_opt else { return Ok(()); };

    grouper::sort_album_tracks(&mut tracks);
    let clean_tracks: Vec<_> = tracks
        .into_iter()
        .map(|(_, mut t)| {
            t.remove("track_path_absolute");
            t
        })
        .collect();

    let (album_pool, track_pools) = compressor::compress(clean_tracks);

    let toml_content = serialize_manifest(&album_pool, &track_pools);
    if toml_content.trim().is_empty() {
        return Ok(());
    }

    if options.stdout {
        println!("{toml_content}");
    } else {
        let out_path = anchor.join("metadata.toml");
        if !options.force && out_path.exists() {
            log::warn!(
                "Existing metadata.toml detected in {}. Use --force to overwrite.",
                anchor.display()
            );
            return Ok(());
        }

        fs::write(&out_path, &toml_content)?;
        log::info!("Generated metadata manifest: {}", out_path.display());
    }

    if !options.stdout {
        write_local_toml(&anchor)?;
    }

    Ok(())
}

fn write_local_toml(anchor: &Path) -> Result<()> {
    let local_toml_path = anchor.join("local.toml");
    if !local_toml_path.exists() {
        let now = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let local_toml_content = format!("[local]\n\ndate_added = {now}\n");
        fs::write(&local_toml_path, local_toml_content)?;
        log::info!("Generated local manifest: {}", local_toml_path.display());
    }
    Ok(())
}

fn find_harvestable_directories(scan_root: &Path, force: bool, supported_exts: &[String]) -> Vec<PathBuf> {
    let mut dirs_to_harvest = Vec::new();
    let mut it = WalkDir::new(scan_root).follow_links(true).into_iter();

    while let Some(Ok(entry)) = it.next() {
        if entry.file_type().is_dir() {
            let path = entry.path();
            if !force && path.join("metadata.toml").exists() {
                it.skip_current_dir();
                continue;
            }

            let has_audio = fs::read_dir(path)
                .is_ok_and(|mut d| {
                    d.any(|e| {
                        if let Ok(f) = e
                            && f.path().is_file()
                                && let Some(ext) = f.path().extension().and_then(|e| e.to_str()) {
                                    let ext_lower = format!(".{}", ext.to_lowercase());
                                    return supported_exts.contains(&ext_lower);
                                }
                        false
                    })
                });

            if has_audio {
                dirs_to_harvest.push(path.to_path_buf());
            }
        }
    }
    dirs_to_harvest
}

fn harvest_audio_files(
    dirs_to_harvest: Vec<PathBuf>,
    supported_exts: &[String],
    options: &ManifestOptions,
) -> Vec<(PathBuf, serde_json::Map<String, serde_json::Value>)> {
    let mut audio_files = Vec::new();
    for dir in dirs_to_harvest {
        let max_depth = match options.mode {
            ManifestMode::Album => 1,
            ManifestMode::Library => usize::MAX,
        };
        for entry in walkdir::WalkDir::new(&dir).max_depth(max_depth).follow_links(true).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.is_file()
                && let Some(ext) = path.extension().and_then(|e| e.to_str()) {
                    let ext_lower = format!(".{}", ext.to_lowercase());
                    if supported_exts.contains(&ext_lower) {
                        audio_files.push(path.to_path_buf());
                    }
                }
        }
    }

    if !options.stdout {
        log::info!("Harvesting {} new audio files...", audio_files.len());
    }

    audio_files
        .into_par_iter()
        .filter_map(|path| match harvest_file(&path) {
            Ok(data) => {
                let mut map = serde_json::Map::new();
                for (k, v) in data.tags {
                    map.insert(k, serde_json::Value::String(v));
                }
                Some((path, map))
            }
            Err(e) => {
                if !options.stdout {
                    log::warn!("Failed to harvest {}: {}", path.display(), e);
                }
                None
            }
        })
        .collect()
}

fn serialize_manifest(
    album_pool: &serde_json::Map<String, serde_json::Value>,
    track_pools: &[serde_json::Map<String, serde_json::Value>],
) -> String {
    let mut toml_content = String::new();
    let album_lines = engine::render_toml_block(album_pool);
    if !album_lines.is_empty() {
        toml_content.push_str("[album]\n");
        toml_content.push_str(&album_lines.join("\n"));
        toml_content.push_str("\n\n");
    }

    for tp in track_pools {
        let track_lines = engine::render_toml_block(tp);
        if !track_lines.is_empty() {
            toml_content.push_str("[[tracks]]\n");
            toml_content.push_str(&track_lines.join("\n"));
            toml_content.push_str("\n\n");
        }
    }

    toml_content
}
