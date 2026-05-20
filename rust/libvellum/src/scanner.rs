use crate::error::VellumError;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

pub fn find_target_albums(path: &Path, max_depth: usize) -> Result<Vec<PathBuf>, VellumError> {
    let mut results = std::collections::HashSet::new();
    if path.join("metadata.toml").exists() || path.join("album.nix").exists() {
        results.insert(path.to_path_buf());
    } else {
        for entry in WalkDir::new(path)
            .max_depth(max_depth)
            .follow_links(true)
        {
            match entry {
                Ok(e) => {
                    if (e.file_name() == "metadata.toml" || e.file_name() == "album.nix")
                        && let Some(parent) = e.path().parent() {
                            results.insert(parent.to_path_buf());
                        }
                }
                Err(e) => {
                    if let Some(io_err) = e.into_io_error() {
                        return Err(VellumError::ManifestIoError(io_err));
                    }
                }
            }
        }
    }
    let mut vec_results: Vec<PathBuf> = results.into_iter().collect();
    vec_results.sort();
    Ok(vec_results)
}

pub fn scan_audio_files(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    let mut files: Vec<PathBuf> = WalkDir::new(root)
        .max_depth(3)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .filter(|p| {
            p.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| {
                    extensions.contains(&format!(".{}", ext.to_lowercase()).as_str())
                })
        })
        .collect();
    files.sort_by(|a, b| alphanumeric_sort::compare_path(a, b));
    files
}
