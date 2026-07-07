# vellum manifest

This command is used to scan audio files in given directory and generate `metadata.toml` and `local.toml` manifests. Has two variants: `album` and `library`.

# vellum manifest album

This variant is used to manifest a new album in provided directory, including all audio files as if they belong to a single album.

## Flags

### --directory / -d
- Provides the directory path for `manifest album` to run in
- Must be a valid directory path
- Defaults to `$(pwd)`

### --extension / -e
- Extension of audio files included for scan
- Must be a string
- Defaults to `flac`

## How It Works
- Scans all audio files with `-e` extension at any depth under `-d`
- Generates `metadata.toml` and `local.toml` in `-d`

# vellum manifest library

This variant is used to manifest all new albums in provided directory, separating audio files into albums by **Exclusivity Principle** (based on grouping keys values), determining directory and generating manifests for each.

## Flags

### --directory / -d
- Provides the directory path for `manifest library` to run in
- Must be a valid directory path
- Defaults to `library_root` value

### --extension / -e
- Extension of audio files included for scan
- Must be a string
- Defaults to `flac`

## How It Works
- Scans all audio files with `-e` extension at any depth under `-d`
- Groups tracks into albums based on grouping keys
- Album group is considered valid if highest level directory path shared by all tracks contains ONLY tracks from this group. If at least one track from any other group is present at any depth in this "common denominator" directory, group is immideately invalidated
- Generates `metadata.toml` and `local.toml` for all valid groups and their "common denominator" directories in `-d`
