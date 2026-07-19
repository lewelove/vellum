use anyhow::Result;
use rayon::prelude::*;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::thread;
use walkdir::WalkDir;

pub use libvellum::harvest::harvest_file;

pub fn run(roots: Vec<PathBuf>, pretty: bool) {
    let extensions = ["flac", "mp3", "m4a", "ogg", "wav", "opus"];
    let mut files = Vec::new();

    for root in roots {
        files.extend(scan_files(&root, &extensions));
    }

    if files.is_empty() {
        return;
    }

    let (tx, rx) = mpsc::channel::<String>();

    let printer_handle = thread::spawn(move || {
        let stdout = io::stdout();
        let mut handle = io::BufWriter::new(stdout.lock());
        for line in rx {
            writeln!(handle, "{line}").ok();
        }
    });

    files.par_iter().for_each_with(tx, |tx, path| {
        if let Ok(payload) = harvest_file(path) {
            let json_res = if pretty {
                serde_json::to_string_pretty(&payload)
            } else {
                serde_json::to_string(&payload)
            };

            if let Ok(json) = json_res {
                tx.send(json).ok();
            }
        }
    });

    printer_handle.join().unwrap();
}

fn scan_files(root: &Path, extensions: &[&str]) -> Vec<PathBuf> {
    if root.is_file() {
        return vec![root.to_path_buf()];
    }

    WalkDir::new(root)
        .follow_links(true)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| e.path().to_path_buf())
        .filter(|p| {
            p.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| extensions.contains(&ext.to_lowercase().as_str()))
        })
        .collect()
}
