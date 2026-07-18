## vl.compile.{level}.key

This function provides config for `"keys": {}` population in `album.lock.json`. It consumes manifests in album folder, creates intermediary `ctx`, calculates the output and returns the value to be written in lock.

### Specifications

#### How function() variables are created:

For `manifests`:
- Resolves the `disknumber` and `tracknumber` for each track.
- Checks treir duplicates so there's no collisions.
- Sorts the `tracks` array by `disknumber` and `tracknumber`.

For `ctx`:
- Finds all audio files at any depth.
- Sorts by file path relative to album root in natural order.
- Populates `tracks` from `[1]` -> total number of files found.
- Appends the `info`, `embedded` and `file` tables into each track

```lua
-- Table containing technical and physical info about concrete files on disk.
local ctx = {
  
  -- Full `vl.config({})` table
  config = {}

  album = {
    id = "", -- Album root directory path relative to `config.storage.library`.
    total_files = 1, -- Total number of audio files.
    duration_milliseconds = 1, -- Sum of all tracks[idx]
  },

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
    -- Each [idx] is sorted & checked by discnumber + tracknumber,
    -- since every [[tracks]] in any manifest must have them,
    -- and expressed in [1] -> total number of [[tracks]].
    tracks = {
      [1] = {},
      [2] = {},
      -- etc...
    }
  }
}
```

#### Album level key specification:

Evaluated once per album:
- Check if `ctx.album.key_name` exists -> if not, check does it exist in all `ctx.tracks[]` and its values are equal for each -> else pass `nil` value
- Check if value passed prom previous step matches the `type` definition (skip if `nil`) -> else invalidate
- If `output = function(value, ctx)` -> run it on the value
- Write output returned value into `[album.lock.json].album.keys.key_name`

```lua
vl.compile.album.key({

  key_name = {

    -- one of Vellum Types
    -- defaults to "object"
    type = "object",

    -- enables the key requirement for lock to compile
    -- if input value OR output returned value is nil or "" throws the compile error
    -- defaults to false
    required = false,

    -- here you can define the function that will perform anything you want to the value
    -- for album level keys output function takes two parameters:
    --   value = ctx.album.{key_name}
    --   ctx = entire album table
    output = function(value, ctx)
      -- default expression
      -- returns itself
      return ctx.album.key_name
    end
  }
})
```

#### Track level key specification:

Evaluated for every individual track in the album separately inside a loop (idx = from 1 to ctx.track_count):
- Check if `ctx.tracks[idx].key_name` exists -> if not, check does `ctx.album.key_name` exist -> if so, inherit -> else pass `nil` value
- Check if `ctx.tracks[idx].key_name` value matches the `type` definition (skip if `nil`) -> else throw error
- If `output = function(value, ctx, idx)` -> run it on the value
- Write output returned value into `[album.lock.json].tracks[idx].keys.key_name`

```lua
vl.compile.tracks.key({

  key_name = {

    -- same as in album level
    type = "object",
    required = false,

    -- inside the tracks level loop output function takes three parameters:
    --   value = ctx.tracks[idx].{key_name}
    --   ctx = entire album table
    --   idx = the index of track in array processed
    output = function(value, ctx, idx)
      -- default expression
      -- for each track returns itself
      return ctx.tracks[idx].key_name
    end
  }
})
```

### Examples

Enables all `key_name_N`:

```lua
-- can be expressed only in separate lines
vl.compile.album.key({ key_name_1 = true })
vl.compile.album.key({ key_name_2 = true })

-- also the "album" -> "a" shorthand can be used
vl.compile.a.key({ key_name_3 = true })

-- "tracks" -> "track" -> "t" with track level keys
vl.compile.tracks.key({ key_name_4 = true })
vl.compile.track.key({ key_name_4 = true })
vl.compile.t.key({ key_name_5 = true })
```

Some other cool examples:

```lua
vl.compile.tracks.key({

  -- wraps title -> `title`
  markdown_monospace_title = {
    output = function(value, ctx, idx)
      local wrap = "`"
      return wrap .. ctx.tracks[idx].title .. wrap
    end
  },

  -- ... add more ...
})
```

Removal of `nil` or empty strings from lock:

```lua
vl.compile.tracks.key({

  -- will compile but will be removed from final lock
  -- reason: output returned nil or "" AND not required
  nil_key = {
    output = function(value, ctx, idx)
      return nil
    end
  },

  -- same with "empty_key"
  empty_key = {
    output = function(value, ctx, idx)
      return ""
    end
  },
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
