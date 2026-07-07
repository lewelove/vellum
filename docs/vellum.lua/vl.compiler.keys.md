## vl.compiler.covers

### Specifications

#### How cover is processed:

- Selects the first valid input -> else throw error
- Links the cover into the `[album.lock.json].album.covers.{name}`

If `cache`:

- Center crops the cover to 1:1
- Resizes to `size` using `filter` algorithm
- Saves to cache

#### Cover specification:

The `vl.compiler.covers({ main = { { "cover.png", "cover.jpg", "folder.png", "folder.jpg" } } })`

```lua
vl.compiler.covers({

  -- used to reference the compiled cover in lock
  name = {

    -- the path of the image relative to metadata.toml that will be targeted by the compiler
    -- can be either the "" or { "", "", ... } array of path strings
    -- if array -> targets the first valid image file path in it
    -- defaults to `{ "cover.png", "cover.jpg", "folder.png", "folder.jpg" }`
    input = { "" },
    
    -- covers cache settings
    -- used to define the parameters of the cached covers for:
    --   unification of cover sizes via `master` (for /api/cover/ calls that miss `static` cache and thus require dynamic rezise source)
    --   further instant disk retrival (for /api/cover/ calls that hit `static` cache)
    -- both master and static intake 2 element arrays (let's call it "cover-config"):
    --   filter = one of resize algorithms provided by rust
    --   size = resize dimensions, must be an integer and > 0
    cache = {

      -- master = single "cover-config"
      -- saves the resulted image into `~/.cache/vellum/covers/{name}/master/{filter}/{size}/{cover_hash}.qoi`
      master = { filter = "", size = 1080 },

      -- static = array of "cover-config"s of any size
      -- each "cover-config" here saves the resulted image into `~/.cache/vellum/covers/{name}/static/{filter}/{size}/{cover_hash}.qoi`
      static = {
        { filter = "", size = 1 },
      },
    },
  },
})
```

#### Default config:

```lua
vl.compiler.covers({
  main = {
    input = {
      "cover.png",
      "cover.jpg",
      "cover.jpeg",
      "front.png",
      "front.jpg",
      "front.jpeg",
      "folder.png",
      "folder.jpg",
      "folder.jpeg",
    },
  },
})
```

## vl.compiler.keys.{level}

This function provides config for `"keys": {}` population in `album.lock.json`. It consumes manifests in album folder, creates intermediary `ctx`, calculates the output and returns the value to be written in lock.

### Specifications

#### How `ctx` is created:

- Merges all avaliable manifests into single struct
- Resolves the `disknumber` and `tracknumber` for each track
- Sorts the `tracks` array by `disknumber` and `tracknumber`
- Appends the `info`, `embedded` and `file` tables into each track

#### Album level key specification:

Evaluated once per album:
- Check if `ctx.album.key_name` exists -> if not, check does it exist in all `ctx.tracks[]` and its values are equal for each -> else pass `nil` value
- Check if value passed prom previous step matches the `type` definition (skip if `nil`) -> else invalidate
- If `output = function(value, ctx)` -> run it on the value
- Write output returned value into `[album.lock.json].album.keys.key_name`

```lua
vl.compiler.keys.album({

  key_name = {

    -- one of Vellum Types
    -- defaults to "object"
    type = "object",

    -- enables the key requirement for lock to compile
    -- if input value OR output returned value is nil or "" throws the compile error
    -- defaults to false
    required = false,

    -- here you can define the funtion that will perform anything you want to the value
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
vl.compiler.keys.tracks({

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
-- can be expressed in one block
vl.compiler.keys.album({
  key_name_1 = true,
  key_name_2 = true,
})

-- or in separate
vl.compiler.keys.album({ key_name_3 = true })
vl.compiler.keys.album({ key_name_4 = true })

-- also the "album" -> "a" shorthand can be used
vl.compiler.keys.a({ key_name_5 = true })

-- same with track level keys
vl.compiler.keys.tracks({ key_name_6 = true })
vl.compiler.keys.t({ key_name_7 = true })
```

Some other cool examples:

```lua
vl.compiler.keys.tracks({

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
vl.compiler.keys.tracks({

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
vl.compiler.keys.tracks({

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
