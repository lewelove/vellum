pub mod compressor;
pub mod engine;
pub mod grouper;

use libvellum::utils::expand_path;
use libvellum::harvest::harvest_file;
use anyhow::{Context, Result};
use rayon::prelude::*;
use std::collections::HashSet;
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
    let manifest_layout = config.app.manifest.keys.as_ref();

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
    let unique_manifests = get_unique_manifests(manifest_layout);

    for (_group_id, tracks) in buckets {
        process_album_group(tracks, &unique_manifests, &supported_exts, manifest_layout, options)?;
    }

    Ok(())
}

fn get_unique_manifests(layout: Option<&indexmap::IndexMap<String, libvellum::lua::config::ManifestKeyConfig>>) -> Vec<String> {
    let mut unique_manifests = HashSet::new();
    unique_manifests.insert("metadata".to_string());
    if let Some(lay) = layout {
        for cfg in lay.values() {
            if let Some(manifests_str) = &cfg.manifests {
                for m in manifests_str.split(',') {
                    let m = m.trim();
                    if !m.is_empty() {
                        unique_manifests.insert(m.to_string());
                    }
                }
            }
        }
    }
    let mut sorted = unique_manifests.into_iter().collect::<Vec<_>>();
    sorted.sort();
    sorted
}

fn process_album_group(
    mut tracks: Vec<(PathBuf, serde_json::Map<String, serde_json::Value>)>,
    manifest_names: &[String],
    supported_exts: &[String],
    manifest_layout: Option<&indexmap::IndexMap<String, libvellum::lua::config::ManifestKeyConfig>>,
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

    for manifest_name in manifest_names {
        let (album_pool, track_pools) =
            compressor::compress(clean_tracks.clone(), manifest_layout, manifest_name);

        let toml_content = serialize_manifest(&album_pool, &track_pools, manifest_layout, manifest_name);
        if toml_content.trim().is_empty() {
            continue;
        }

        if options.stdout {
            if manifest_names.len() > 1 {
                println!("--- {manifest_name}.toml ---");
            }
            println!("{toml_content}");
        } else {
            let out_path = anchor.join(format!("{manifest_name}.toml"));
            if !options.force && out_path.exists() {
                log::warn!(
                    "Existing {manifest_name}.toml detected in {}. Use --force to overwrite.",
                    anchor.display()
                );
                continue;
            }

            fs::write(&out_path, &toml_content)?;
            log::info!("Generated {manifest_name} manifest: {}", out_path.display());
        }
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
                .map(|mut d| {
                    d.any(|e| {
                        if let Ok(f) = e
                            && f.path().is_file()
                                && let Some(ext) = f.path().extension().and_then(|e| e.to_str()) {
                                    let ext_lower = format!(".{}", ext.to_lowercase());
                                    return supported_exts.contains(&ext_lower);
                                }
                        false
                    })
                })
                .unwrap_or(false);

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
    manifest_layout: Option<&indexmap::IndexMap<String, libvellum::lua::config::ManifestKeyConfig>>,
    target_manifest: &str,
) -> String {
    let mut toml_content = String::new();
    let album_lines = engine::render_toml_block(album_pool, manifest_layout, "album", target_manifest);
    if !album_lines.is_empty() {
        toml_content.push_str("[album]\n");
        toml_content.push_str(&album_lines.join("\n"));
        toml_content.push_str("\n\n");
    }

    for tp in track_pools {
        let track_lines = engine::render_toml_block(tp, manifest_layout, "track", target_manifest);
        if !track_lines.is_empty() {
            toml_content.push_str("[[tracks]]\n");
            toml_content.push_str(&track_lines.join("\n"));
            toml_content.push_str("\n\n");
        }
    }

    toml_content
}
