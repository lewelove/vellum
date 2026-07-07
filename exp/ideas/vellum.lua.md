# vellum.lua

This file describes and outlines future documentation of vellum configuration written in lua.

---

The `~/.config/vellum/vellum.lua` is the initial file that must exist for config to be active.

## require()

For modularizing config use `require("name")` of the `name.lua` files reative to `~/.config/vellum/`. For path specification use `.` as the delimiter.

```lua

-- imports ~/.config/vellum/module.lua
require("module")

-- imports ~/.config/vellum/folder/module.lua
require("folder.module")
```

## vl.config

Function that returns the static config struct.

```lua

vl.config({

  storage = {
    -- path to library root containing all your albums
    library = "",
    -- path to .env file vellum will load for actions execution
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


## vl.compiler.covers

```lua
-- center crops the cover to 1:1
-- resizes to `size` using `interpolation` algorithm
-- saves to cache
vl.compiler.covers( "name", {
  -- resize algorithm
  interpolation = "",
  -- resize dimensions
  size = 1,
})
```

## vl.compiler.keys.{level}

This function provides config for `"keys"` population in `album.lock.json`. It consumes manifests in album folder, creates intermediary `ctx`, and returns the value to be written in lock.

### How it works

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

If `output = true` (default value if `output` is omitted or `nil`)
- Return the `input` value unchanged

If `output = false`
- Skip the key evaluation entirely

If `output = function(value, ctx, idx)`
- First argument is the value from `ctx` object resolved via rust
- Second argument is the entire `ctx` of the album
- Third argument is optional and passed only if `level = "track"`. It always equals to the array index of the track processed

Else throw config error

### Examples

Enables all `key_name_N`, falling back to `type = "object"` and `output = true`.

```lua

-- can be expressed in one block

vl.compiler.keys.album({
  key_name_1 = true
  key_name_2 = true
})

-- or in separate

vl.compiler.keys.album({ key_name_3 = true })
vl.compiler.keys.album({ key_name_4 = true })

-- also the "album" -> "a" shorthand can be used

vl.compiler.keys.a({ key_name_5 = true })

-- same with track level keys

vl.compiler.keys.track({ key_name_6 = true })
vl.compiler.keys.t({ key_name_7 = true })

```

Enables album and track level keys and sets their parameters.

```lua

vl.compiler.keys.album({

  some_cool_key = {

    -- one of Vellum Types
    -- defaults to "object"
    type = "object",

    -- enables the key requirement for album to compile
    -- if input value is nil throws the compile error
    -- defaults to false
    required = false,

    -- here you can define the funtion that will perform anything you want to the value
    output = function(value, ctx)
      -- default expression
      -- returns itself
      return ctx.album.some_cool_key
    end
  }

})

vl.compiler.keys.album({

  -- "empty_key" will exist in album.lock.json but will always contain an empty string
  empty_key = { output = function(v, ctx) return "" end }

  -- ...more examples...

})

vl.compiler.keys.tracks({

  -- wraps title -> `title`
  markdown_monospace_title = {
    output = function(value, ctx, idx)
      local wrap = "`"
      return wrap .. ctx.tracks[idx].title .. wrap
    end
  }

  -- will always throw a compile error
  -- reason: output returned nil
  -- useful with conditionals
  error! = {
    output = function(value, ctx, idx)
      return nil
    end
  }

  -- ...more examples...

})

```

## vl.interfaces

This function provides the data for `vellum interface` to run.

```lua

vl.interfaces({

  -- the `default` interface name
  -- run with `vellum interface` or `vellum interface default`
  -- always exists, can be overwritten
  default = {

    -- path to override expected executable interface binary
    -- defaults to `~/.local/share/vellum/interfaces/{name}/run.sh`
    run = "",

    -- the config table
    -- can be populated with any static data
    -- converted to JSON and sent as `/api/interfaces/{name}/config` endpoint
    config = {}, 

    assets = {}
  }
})
```

The `{name} = true` can be used to enable interface with default run path and no config

```lua

vl.interfaces({
  -- run with `vellum interface simple_ui`
  -- inherits `run = "~/.local/share/vellum/interfaces/simple_ui/run.sh"`
  -- no config api
  simple_ui = true
})
```

Also the `config = {}` and `run = "path/to/run"` can be individually provided to enable it

```lua

vl.interfaces({
  -- run with `vellum interface interface_name`
  -- inherits `run = "~/.local/share/vellum/interface_name/UwU/run.sh"`
  -- `/api/interfaces/interface_name/config` will return `{ "theme": "dark" }` JSON
  interface_name = { config = { theme = "dark" } }
})
```

```lua

local repo_path = "~/dev/vellum/"

vl.interfaces({
  -- run with `vellum interface interface_name`
  -- no config api
  interface_name = { run = repo_path .. "interfaces/example/cool_script.sh" }
})
```

## vl.actions

This function provides the data for `vellum x` to run

```lua

vl.actions({

  action_name = {

    -- path to override expected executable action binary
    -- defaults to `~/.local/share/vellum/actions/{name}/bin.sh`
    run = "",

    -- the config table
    -- can be populated with any static data
    -- converted to JSON and sent as a part of stdin payload
    config = {}, 
  }

})
```

