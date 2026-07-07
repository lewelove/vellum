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
