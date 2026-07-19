use anyhow::{Context, Result};
use lofty::config::ParseOptions;
use lofty::file::AudioFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use lofty::tag::TagType;
use serde::Serialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
pub struct TrackJson {
    pub path: PathBuf,
    pub tags: HashMap<String, String>,
    pub physics: PhysicsData,
}

#[derive(Serialize)]
pub struct PhysicsData {
    pub file_size: u64,
    pub mtime: u64,
    pub duration_ms: u64,
    pub sample_rate: u32,
    pub bit_depth: Option<u8>,
    pub channels: u8,
    pub audio_bitrate: u32,
    pub overall_bitrate: u32,
    pub format: String,
}

#[must_use] 
pub fn sanitize_key(key: &str) -> String {
    let mut out = String::new();
    let mut last_was_under = false;
    for c in key.chars() {
        if c.is_ascii_alphanumeric() {
            out.push(c.to_ascii_lowercase());
            last_was_under = false;
        } else if !last_was_under {
            out.push('_');
            last_was_under = true;
        }
    }
    out.trim_matches('_').to_string()
}

pub fn harvest_file(path: &Path) -> Result<TrackJson> {
    let metadata = fs::metadata(path)?;
    let mut f = std::fs::File::open(path).context("Open failed")?;
    let probe = Probe::new(&mut f).guess_file_type().context("Guess failed")?;
    let file_type = probe.file_type();

    let tagged_file = Probe::open(path)?
        .options(ParseOptions::new().read_cover_art(false))
        .read()
        .context("Read failed")?;

    if file_type == Some(lofty::file::FileType::Flac)
        && tagged_file.tag(TagType::Id3v2).is_some() {
            log::warn!(
                "ID3v2 tag encountered in FLAC (incompatible with standards): {}",
                path.display()
            );
        }

    let physics = extract_physics(&metadata, &tagged_file);

    let mut tags = HashMap::new();
    let concrete_parsed = extract_concrete_tags(path, file_type, &mut tags)?;

    if !concrete_parsed {
        extract_fallback_tags(&tagged_file, &mut tags);
    }

    Ok(TrackJson {
        path: path.to_path_buf(),
        tags,
        physics,
    })
}

fn extract_physics(metadata: &std::fs::Metadata, tagged_file: &lofty::file::TaggedFile) -> PhysicsData {
    let file_size = metadata.len();
    let mtime = metadata
        .modified()
        .unwrap_or(std::time::SystemTime::UNIX_EPOCH)
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let properties = tagged_file.properties();

    PhysicsData {
        file_size,
        mtime,
        duration_ms: u64::try_from(properties.duration().as_millis()).unwrap_or(u64::MAX),
        sample_rate: properties.sample_rate().unwrap_or(0),
        bit_depth: properties.bit_depth(),
        channels: properties.channels().unwrap_or(0),
        audio_bitrate: properties.audio_bitrate().unwrap_or(0),
        overall_bitrate: properties.overall_bitrate().unwrap_or(0),
        format: format!("{:?}", tagged_file.file_type()),
    }
}

fn extract_concrete_tags(
    path: &Path,
    file_type: Option<lofty::file::FileType>,
    tags: &mut HashMap<String, String>,
) -> Result<bool> {
    let mut file_content = std::fs::File::open(path)?;
    let mut concrete_parsed = false;
    match file_type {
        Some(lofty::file::FileType::Flac) => {
            if let Ok(flac) = lofty::flac::FlacFile::read_from(
                &mut file_content,
                ParseOptions::new().read_cover_art(false),
            )
                && let Some(comments) = flac.vorbis_comments() {
                    parse_vorbis_comments(comments.items(), tags);
                    concrete_parsed = true;
                }
        }
        Some(lofty::file::FileType::Vorbis) => {
            if let Ok(ogg) = lofty::ogg::VorbisFile::read_from(
                &mut file_content,
                ParseOptions::new().read_cover_art(false),
            ) {
                parse_vorbis_comments(ogg.vorbis_comments().items(), tags);
                concrete_parsed = true;
            }
        }
        Some(lofty::file::FileType::Opus) => {
            if let Ok(opus) = lofty::ogg::OpusFile::read_from(
                &mut file_content,
                ParseOptions::new().read_cover_art(false),
            ) {
                parse_vorbis_comments(opus.vorbis_comments().items(), tags);
                concrete_parsed = true;
            }
        }
        _ => {}
    }
    Ok(concrete_parsed)
}

fn parse_vorbis_comments<'a, I>(items: I, tags: &mut HashMap<String, String>)
where
    I: Iterator<Item = (&'a str, &'a str)>,
{
    for (k, v) in items {
        let key = sanitize_key(k);
        let value = v.trim();
        if !key.is_empty() && !value.is_empty() {
            tags.entry(key)
                .and_modify(|e: &mut String| {
                    if !e.contains(value) {
                        e.push_str("; ");
                        e.push_str(value);
                    }
                })
                .or_insert_with(|| value.to_string());
        }
    }
}

fn extract_fallback_tags(tagged_file: &lofty::file::TaggedFile, tags: &mut HashMap<String, String>) {
    if let Some(tag) = tagged_file
        .primary_tag()
        .or_else(|| tagged_file.first_tag())
    {
        let tag_type = tag.tag_type();
        for item in tag.items() {
            let key_raw = item
                .key()
                .map_key(tag_type).map_or_else(|| format!("{:?}", item.key()), ToString::to_string);
            let key = sanitize_key(&key_raw);

            let Some(value) = item.value().text() else {
                continue;
            };
            let value = value.trim();

            if key.is_empty() || value.is_empty() {
                continue;
            }

            tags.entry(key)
                .and_modify(|existing: &mut String| {
                    if !existing.contains(value) {
                        existing.push_str("; ");
                        existing.push_str(value);
                    }
                })
                .or_insert_with(|| value.to_string());
        }
    }
}
