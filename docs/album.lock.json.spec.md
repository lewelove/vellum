# album.lock.json

This document describes specification of the `album.lock.json` file, how it compiles and what fields mean.

```jsonc
{
  // type definitions
  // values below are used to determine type of vellum value for each lock file key
  //
  // "" -> string
  // 1 -> number
  // [] -> array of strings
  // "dt" -> datetime
  // 0.1 -> float
  // "/" -> path
  // true -> boolean

  "album": {

    // Three album level keys resolved by Rust that are required for album to be legible,
    // the inability of computing them (missing or wrong) throws compile error. All of them
    // must exist in `metadata.toml` in some way.

    // Either `album.albumartist` or `album.artist`, and be a string.
    "albumartist": "",
    // The `album.album` value, a string.
    "album": "",
    // The `album.date` value, a string.
    "date": "",

    // Album path relative to `vl.config.storage.library`.
    "id": ""

    // User configurable keys from `vl.compile.album.key`.
    "keys": {},

    "info": {

      "total_discs": 1,
      "total_tracks": 1,

      // Datetime from `[local.toml].local.date_added`.
      "date_added": "dt",

      // Total sum of tracks[].duration.
      "duration_milliseconds": 0,

      // `duration_milliseconds` -> Formatted to "HH:MM:SS".
      "duration_formatted": ""

    }

    // Object with all `{name}.toml` manifests used to generate `album.lock.json`.
    "manifests": {
      "metadata": { // `metadata.toml`, always present.
        "file": {
          "path": "/", // Relative to album.info.id file path.
          "mtime": 1, // File mtime.
          "byte_size": 1 // File size in bytes.
        }
      },
      "local": { // `local.toml`
        "file": {
          "path": "/",
          "mtime": 1,
          "byte_size": 1
        }
      },
      "name": { // `{name}.toml`, an example.
        "file": {
          "path": "/",
          "mtime": 1,
          "byte_size": 1
        }
      },
      // etc...
    },

    "covers": {

      // Main cover derived from cover.png / cover.jpg
      "main": {

        "file": {
          "path": "",

          // Present only for "covers".
          "address": "" // file.hash -> Raw -> No-pad -> Truncated to 16 chars.
          "hash": "blake3-{hash}=", // file.path -> BLAKE3 hash in SRI Base64 format.

          "mtime": 1,
          "byte_size": 1
        }
      }
    }
  },
  "tracks": [
    {
      // Four tracks level keys resolved by Rust from `metadata.toml` as well.
      // Must be satisfied, else throw compile error.

      "discnumber": 1,
      "tracknumber": 1,

      // Either `tracks[].artist` or `album.albumartist` or `album.artist`
      "artist": "",
      // `tracks[].title`
      "title": "",

      // User keys provided by `vl.compile.tracks.key`
      "keys": {},
      "info": {
        "sample_rate": 1,
        "bits_per_sample": 1,
        "bitrate_kbps": 1,
        "encoding": "",
        "channels": 1,
        "duration_milliseconds": 1,
        "duration_formatted": "",
      },
      "file": {
        "path": "",
        "mtime": 1,
        "byte_size": 1,
      }
      "lyrics": {
        "file": {
          "path": "",
          "mtime": 1,
          "byte_size": 1
        },
      },
    },
    {
      // etc...
    }
  ]
}
```

