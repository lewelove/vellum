use anyhow::{Context, Result};
use globset::{Glob, GlobSet, GlobSetBuilder};

pub fn build_globset(filter: &str) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    
    for part in filter.split(',') {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        
        let pattern = if !trimmed.contains('/') && !trimmed.contains('*') && !trimmed.contains('?') {
            let ext = trimmed.trim_start_matches('.');
            format!("**/*.{ext}")
        } else {
            trimmed.to_string()
        };
        
        builder.add(
            Glob::new(&pattern)
                .with_context(|| format!("Invalid glob pattern: {pattern}"))?
        );
    }
    
    builder.build().context("Failed to build GlobSet")
}
