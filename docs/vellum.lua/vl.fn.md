# vl.fn

This document describes a built-in set of `vl.fn` functions that can be used *inside* `function()` provided by any `vl.compile` to execute useful logic without lots of Lua boilerplate.

**Specifications:**
- For all `fn` consuming a system path string Rust canonicalizes the system path. If string is passed as relative path it joins it to `__VELLUM_ACTIVE_ROOT` variable (the root of album being compiled).

### vl.fn.type_check

Checks input value (usually provided by `manifests` table) for one of Vellum Types, else throws error. Can be used on empty (nil or "") values, which will pass it, as this logic is delegated to `vl.fn.require()`.

```lua
vl.fn.type_check(value, "vellum_type")
```

### vl.fn.require

Takes Lua value and checks if the value passed is `nil` or `""`. If false returns value. If true throws compilation error.

```lua
vl.fn.require(value)
```

### vl.fn.hash_string

Takes a string as input and returns a BLAKE3 hash of string itself. Useful for static hash generation from album data, for specific sort for example.

```lua
vl.compile.album.key({ cool_id = function(ctx, m)
  local key = m.metadata.album.album .. m.metadata.album.albumartist .. m.metadata.album.date
  return vl.fn.hash_string(key)
end })
```

### vl.fn.hash_file

Takes a system path string as input and returns a BLAKE3 hash of the file. Cannot be a directory.

```lua
vl.compile.album.key({ cool_id = function(ctx, m)
  local key = m.metadata.album.album .. m.metadata.album.albumartist .. m.metadata.album.date
  return vl.fn.hash_string(key)
end })
```

### vl.fn.to_table

Takes system path string of a JSON/TOML file as input and returns Lua table. Useful for pulling non-manifest data files to the compilation context.

**Specifications:**
- Reads the `.ext` of the file, matches the JSON/TOML based on it. Validates the file.
- Converts to Lua table, returns it.

### vl.fn.read_text

Takes the system path string of any file and returns a literal string. Useful for pulling literally any text data to the compilation context, like lyrics or notes.
