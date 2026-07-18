# vl.compile.{level}.key

This function provides config for `"keys": {}` population in `album.lock.json`. It consumes manifests in album folder, creates intermediary `ctx`, calculates the output and returns the value to be written in lock.

## Specifications

### How function() variables are created:

For `ctx`:
- Finds all audio files at any depth.
- Sorts by file path relative to album root in natural order.
- Populates `tracks` from `[1]` -> total number of files found.
- Appends the `embedded` and `file` tables into each track.

For `manifests.metadata`:
- Reads `metadata.toml`.
- Populates `album` table with `[album]` directly.
- Resolves the `disknumber` and `tracknumber` for each `[[tracks]]` (at least 1 must exist).
- Checks for tuple duplicates so there's no collisions.
- Sorts tuples by `disknumber` and `tracknumber`.
- For each unique tuple the `[idx]` is given from `[1]` to the total number of `[[tracks]]` found.
- Populates `tracks[idx]` with `[[tracks]]` data.

For each next `manifests.<manifest_name>`:
- Reads `<manifest_name>.toml`.
- Populates `album` table with `[album]` directly.
- Resolves the `disknumber` and `tracknumber` for each `[[tracks]]`.
- Checks for tuple duplicates so there's no collisions.
- Sorts tuples by `disknumber` and `tracknumber`.
- Checks the total number of `[[tracks]]` found -> is either zero or equal to their number in `manifests.metadata`.
- For each unique tuple the `[idx]` is given from `[1]` to the total number of `[[tracks]]` found.
- Populates `tracks[idx]` with `[[tracks]]` data.

Before function execution Rust makes sure the number of `tracks` are equal in both `ctx` and `manifests` that have them. Else throw a compile error.

```lua
-- Table containing technical and physical info about concrete files on disk.
local ctx = {
  
  -- Full `vl.config({})` table
  config = {}

  id = "", -- Album root directory path relative to `config.storage.library`.
  total_files = 1, -- Total number of audio files.
  duration_milliseconds = 1, -- Sum of all tracks[idx]

  -- For every audio file found at any depth -> 
  tracks = {
    [1] = {
      sample_rate = 1,
      bit_depth = 1,
      bitrate_kbps = 1,
      encoding = "",
      channels = 2,
      duration_milliseconds = 1,

      file = {
        path = "", -- Audio file path relative to album root.
        mtime = 1,
        byte_size = 1,
      },
      -- Table containing `key = "string_value"` resolved directly
      -- from audio file embedded tags. All `key` are sanitized by Rust:
      --   - Allow only [a-z] and [0-9]
      --   - Replace everything else with _
      --   - To lowercase
      --   - Make sure there's no collisions in resulted table
      embedded = {
      }
    },
    [2] = {},
    -- etc...
  }
}

-- Table containing raw toml manifests -> Lua tables.
local manifests = {
  -- For every <manifest>.toml found in album folder create literal <manifest> = {} Lua table.
  metadata = { -- Always present since metadata.toml is required.
    album = {
    },
    tracks = {
      [1] = {},
      [2] = {},
      -- etc...
    }
  }
}
```

### Helper Functions

There is a built-in set of `vl.fn` functions that can be used *inside* `key_name = function()` to execute useful logic without lots of Lua boilerplate.

#### vl.fn.type_check

Checks input value (usually provided by `manifests` table) for one of Vellum Types, else throws error.

```lua
vl.fn.type_check(value, "vellum_type")
```

#### vl.fn.require

Checks if the value passed is `nil` or not. If `nil` throws error.

```lua
vl.fn.require(value)
```

### Album level key specification:

The key name provided must be always a `function()` that returns value. For `album` level this function consumes two arguments: `ctx` and `manifests`. The function is evaluated once per album.

#### Examples:

```lua
-- Simply returns the key from primary manifest
vl.compile.album.key({
  key_name = function(ctx, manifests)
    return manifests.metadata.album.key_name
  end
})
```

The `album` shorthands:

```lua
-- "album" -> "a"
vl.compile.a.key({ key_name = function(ctx, m) return "I am truncated!" end })
```

### Track level key specification:

For `tracks` level key name must be a function that returns value, just like in `album`. The only difference is that it evaluates for each individual track separately, and thus require the additional `idx` argument, that is always equal to `idx` of track evaluated.

#### Examples:

```lua
-- Simply returns the key for each track from primary manifest
vl.compile.album.key({
  key_name = function(ctx, manifests, idx)
    return manifests.metadata.tracks[idx].key_name
  end
})
```

The `tracks` shorthands:

```lua
-- "tracks" -> "track" -> "t"
vl.compile.track.key({ key_name = function(ctx, m, i) return "hi" end  })
vl.compile.t.key({ key_name = function(ctx, m, i) return "thanks for reading docs..." end })
```

### More Examples:

Each function can be expressed only in separate blocks.

```lua
vl.compile.album.key({ key_name_1 = function(ctx, m) return "Hello, World!" end })
vl.compile.tracks.key({ key_name_2 = function(ctx, m, i) return 5318008 end })
```

Invalid syntax. Each `vl.compile.{level}.key({})` must contain single element only.

```lua
vl.compile.album.key({ 
  -- Forbidden!
  key_name_1 = function(ctx, m) return "Oh, No!" end
  key_name_2 = function(ctx, m) return ":(" end
})
```

Some other cool examples:

```lua
-- Type check for string, require tag to exist in id.toml manifest
vl.compile.album.key({
  musicbrainz_release_url = function(ctx, manifests)
    local id = manifests.id.album.musicbrainz_releaseid
    local url = "https://musicbrainz.org/release/"
    vl.fn.require(id) -- `album.musicbrainz_releaseid` in `id.toml` must exist
    vl.fn.type_check(id, "string") -- `album.musicbrainz_releaseid` in `id.toml` must be a string
    return url .. id
    -- Result:
    -- [album.lock.json].album.keys.musicbrainz_release_url = "https://musicbrainz.org/release/{id}"
  end
})
```

```lua
-- Wraps title -> `title`
vl.compile.tracks.key({
  markdown_monospace_title = function(ctx, m, i)
    local wrap = "`"
    local title = m.tracks[i].title
    return wrap .. title .. wrap
  end
})
```

Work in progress...

If value returned was `nil` or an empty string it will be removed from lock

```lua
-- Will compile but will be removed from final lock
vl.compile.album.key({
  nil_key = function(ctx, m)
    return nil
  end
})
-- Same with "empty_key"
vl.compile.tracks.key({
  empty_key = function(ctx, m, i)
    return ""
  end
})
```

This will never compile:

```lua
vl.compile.tracks.key({

  -- will always throw a compile error
  -- reason: output returned nil AND is required
  error_key = {
    required = true,
    output = function(value, ctx, idx)
      return nil
    end
  },
})
```
