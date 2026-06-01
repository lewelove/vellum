use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VellumError {
    #[error("Missing primary manifest: metadata.toml not found in {path}")]
    MissingPrimaryManifest { path: PathBuf },

    #[error("IO Error: {0}")]
    ManifestIoError(#[from] std::io::Error),

    #[error("Parse Error: Failed to parse TOML in {path}: {source}")]
    ManifestParseError { path: PathBuf, source: toml::de::Error },

    #[error("Serialization Error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Schema Error: [[tracks]] array not found in metadata.toml for {path}")]
    MissingTracksBlock { path: PathBuf },

    #[error("Manifest Structure Violation: Auxiliary manifest '{manifest}' in {path} defines {aux_count} tracks, but metadata.toml defines {primary_count} tracks. Every manifest must describe the exact same track count.")]
    TrackCountMismatch { manifest: String, path: PathBuf, primary_count: usize, aux_count: usize },

    #[error("Manifest '{manifest}' at {path} has an invalid entry at index {index}: Track entries must be TOML tables.")]
    InvalidManifestEntry { manifest: String, path: PathBuf, index: usize },

    #[error("Inventory Mismatch in {path}: Found {files_count} audio files on disk but metadata.toml defines {tracks_count} tracks. Vellum requires an explicit 1:1 mapping between disk files and metadata entries.")]
    PhysicalInventoryMismatch { path: PathBuf, files_count: usize, tracks_count: usize },

    #[error("Identity Error: Manifest '{manifest}' in {path} is missing or has an invalid TRACKNUMBER field at track index {index}")]
    MissingTrackIdentity { manifest: String, path: PathBuf, index: usize },

    #[error("{message} for field '{field}'")]
    InvalidIdentityFormat { field: String, message: String },

    #[error("Identity Collision: Manifest '{manifest}' in {path} defines Disc {disc}, Track {track} more than once.")]
    DuplicateTrackIdentity { manifest: String, path: PathBuf, disc: u32, track: u32 },

    #[error("Orphaned Track Data: Manifest '{manifest}' in {path} contains data for Disc {disc}, Track {track}, but no such track exists in the primary metadata.toml.")]
    OrphanedAuxiliaryData { manifest: String, path: PathBuf, disc: u32, track: u32 },

    #[error("Integrity Violation in {path}: Key '{key}' is restricted to 'album' level, but conflicting values were detected ('{val1}' in {source1} versus '{val2}' in track {index}). All track-level overrides must match the album-level value.")]
    GlobalKeyConflict { 
        path: Box<PathBuf>, 
        key: Box<String>, 
        val1: Box<String>, 
        source1: Box<String>, 
        val2: Box<String>, 
        index: usize 
    },

    #[error("Configuration Error: [compiler.keys] block is missing from config.toml")]
    MissingCompilerRegistry,

    #[error("Harvest Error: Failed to harvest metadata from audio file {path}: {source}")]
    HarvestError { path: PathBuf, source: anyhow::Error },

    #[error("Type Mismatch in {path}: Field '{key}' is expected to be a '{expected_type}', but found value: {found_val}")]
    TypeMismatch {
        path: PathBuf,
        key: String,
        expected_type: String,
        found_val: String,
    },
}
