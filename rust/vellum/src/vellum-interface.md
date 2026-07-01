# `vellum interface`

This command is used to run system installed interfaces for vellum from cli

## Arguments

### `<NAME>`
- Resolves `[config].interfaces.<NAME>.run` and executes it
- If omitted -> "default"

Notes:
- The `[config].interfaces.default.enable` always resolves to `true`, this way if `<NAME>` argument is ommited we can use flags without interface being registered in config

## Examples

```bash
# inherits:
#   directory = "~/.local/share/vellum/interfaces/default"
#   config    = "~/.local/share/vellum/interfaces/default/config.toml"
#   run       = "~/.local/share/vellum/interfaces/default/run.sh"
vellum interface
```

```bash
# if [config].interfaces.web-app.enable -> inherits:
#   directory = "~/.local/share/vellum/interfaces/web-app"
#   config    = "~/.local/share/vellum/interfaces/web-app/config.toml"
#   run       = "~/.local/share/vellum/interfaces/web-app/run.sh"
vellum interface web-app
```

```bash
# if [config].interfaces.web-app.enable AND [config].interfaces.web-app.config = {path} -> inherits:
#   directory = "~/.local/share/vellum/interfaces/web-app"
#   config    = "{path}"
#   run       = "~/.local/share/vellum/interfaces/web-app/run.sh"
vellum interface web-app
```

## New Config Attributes

Server builds struct based on this table, with automatic population of missing attributes

```toml

[interfaces]

# interface name
default = {

  # requried to register interface for vellum cli
  enable = true,

  # path to override folder where expected vellum.toml lies
  # defaults to `~/.local/share/vellum/interfaces/{name}/`
  # used to default other paths againts to
  directory = "",

  # path to any .toml file that is converted to json and sent at /api/interfaces/{name}/config request
  # defaults to `~/.local/share/vellum/interfaces/{name}/config.toml` 
  config = "",

  # path to executable file `vellum interface {name}` runs
  # defaults to `~/.local/share/vellum/interfaces/{name}/run.sh`
  run = ""

}

```

## API

### /api/interfaces/{name}/config

This API endpoint is used to retrieve `toml` file intended for interface configuration from resolved `[~/.config/vellum/config.toml].interfaces.{name}.config` path, convert it to `json` and POST it to websocket. The resolved config path is watched by `vellum server` inotify and hot reloads it.

## todo

change `vellum ui` to `vellum interface`

make interfaces reside in `./interfaces/`, make current `web-app/` ui reside in it

provide `web-app/config.toml` where we define all configurations separated from vellum binary itself

provide in `~/.config/vellum/config.toml`
