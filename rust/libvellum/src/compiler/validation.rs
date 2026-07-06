use crate::error::VellumError;
use crate::compiler::manifest::extract_strict_u32;
use serde_json::Value;
use std::collections::HashSet;
use std::path::Path;

pub fn validate_track_indices(entries: &[Value], root: &Path) -> Result<(), VellumError> {
    let mut seen_ids = HashSet::new();
    for (idx, entry) in entries.iter().enumerate() {
        let t = extract_strict_u32(entry.get("tracknumber"), "tracknumber", None)
            .map_err(|_| VellumError::MissingTrackIdentity {
                manifest: "metadata.toml".to_string(),
                path: root.to_path_buf(),
                index: idx + 1,
            })?;
        let d = extract_strict_u32(entry.get("discnumber"), "discnumber", Some(1))?;

        if !seen_ids.insert((d, t)) {
            return Err(VellumError::DuplicateTrackIdentity {
                manifest: "metadata.toml".to_string(),
                path: root.to_path_buf(),
                disc: d,
                track: t,
            });
        }
    }
    Ok(())
}
