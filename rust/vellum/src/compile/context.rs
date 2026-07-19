use libvellum::utils::expand_path;
use std::path::{Path, PathBuf};

pub struct PreparedContext {
    pub audio_files: Vec<PathBuf>,
    pub library_root: PathBuf,
}

pub fn prepare_build_context(
    config: &libvellum::lua::ResolvedConfig,
    album_root: &Path,
) -> PreparedContext {
    let exts: Vec<String> = config.app.manifest.audio_files.clone().unwrap_or_else(|| vec![".flac".to_string()]);
    let ext_refs: Vec<&str> = exts.iter().map(AsRef::as_ref).collect();
    let audio_files = libvellum::scanner::scan_audio_files(album_root, &ext_refs);

    let lib_root_raw = &config.app.storage.library;
    let library_root = expand_path(lib_root_raw)
        .canonicalize()
        .unwrap_or_else(|_| expand_path(lib_root_raw));

    PreparedContext { audio_files, library_root }
}
