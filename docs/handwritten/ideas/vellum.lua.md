# vellum.lua

This file describes and outlines future documentation of vellum configuration written in lua.

---

The `~/.config/vellum/vellum.lua` is the initial file that must exist for config to be active.

## require()

For modularizing config use `require("name")` of the `name.lua` files reative to `~/.config/vellum/`. For path specification use `.` as the delimiter.

```lua

-- Imports ~/.config/vellum/module.lua
require("module")

-- Imports ~/.config/vellum/folder/module.lua
require("folder.module")
```

## vl.config

Function that returns the static config struct.

```lua

vl.config({

  storage = {
    -- Path to library root containing all your albums
    library = "",
    -- Path to .env file vellum will load for actions execution
    environment = "",
  },

  manifest = {
    audio_files = { "flac" },
  },

  compiler = {
  },

})
```

## vl.fn

Set of builtin rust functions to call at compile or action time.

## vl.compile

Sets of config for functions that run at compile time for each album separately.

### vl.compile.cover

```lua
-- Center crops the cover to 1:1
-- Resizes to `size` using `interpolation` algorithm
-- Saves to cache
vl.compile.cover( "name", {
  -- Used for reference, can be omitted
  name = "",
  -- Resize algorithm
  interpolation = "",
  -- Resize dimensions
  size = 1,
})
```

### vl.compile.key

This fucntion provides config for `"keys"` population in `album.lock.json`. It consumes manifests in album folder, creates intermediary `ctx`, and returns the value to be written in lock.

How `ctx` is created:

- Merges all avaliable manifests into single struct
- Resolves the `disknumber` and `tracknumber` for each track
- Sorts the `tracks` array by `disknumber` and `tracknumber`
- Appends the `info`, `embedded` and `file` tables into each track

Then for each registered `vl.compile.key( "key_name", { ... })`:

If `level = "album"`
- Check if `ctx.album.key_name` exists -> if not, check does it exist in all `ctx.tracks[]` and its values are equal for each -> else pass `nil` value
- Check if value passed prom previous step matches the `type` definition (skip if `nil`) -> else invalidate
- Run `output = function(value, ctx)` on the value
- Write output returned value into `[album.lock.json].album.keys.key_name`

If `level = "track"`, for index (`idx`) of each track in array separately
- Check if `ctx.tracks[idx].key_name` exists -> if not, check does it exist in `ctx.album` -> if so, inherit -> else pass `nil` value
- Check if `ctx.tracks[idx].key_name` value matches the `type` definition (skip if `nil`) -> else invalidate
- Run `output = function(value, ctx, idx)` on the value
- Write returned value into `[album.lock.json].tracks[idx].keys.key_name`

The `output` spec:

If `output = true`
- Return the `input` value unchanged
- The default value if `output` is omitted or `nil`

If `output = false`
- Skip the key evaluation entirely

If `output = function(value, ctx, idx)`
- First argument is the value from `ctx` object resolved via rust
- Second argument is the entire `ctx` of the album
- Third argument is optional and passed only if `level = "track"`. It always equals to the array index of the track processed

Else throw config error

```lua
vl.compile.key( "key_name", {

  -- Determines the level of where the key will be written
  level = "album" | "track",

  -- One of Vellum Types
  type = "string" | "integer" | "boolean" | "array" | "datetime" | "url",

  -- Here you can define the funtion that will perform anything you want to the value
  output = function(value, ctx, idx)
    return ""
  end

})
```


