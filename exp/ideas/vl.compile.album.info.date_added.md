# vl.compile.album.info.date_added

Is embedded Lua function (can be overridden in config) that provides the calculation of the `[album.lock.json].album.info.date_added` value.

## How it works

The value is read from `system.toml` manifest, provided in `function(ctx, m)`. With fallbacks:
- date_added
- date_generated

The function throws an error if no `album.system.date_generated` or `album.system.date_added` can be found in it.

```lua
vl.compile.album.info.date_added( function(ctx, m)
  -- populate the logic here
end )
```
