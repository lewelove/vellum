# vellum x

## How it works

The `vellum x` command is used to run **Vellum Actions** for selected albums or the entire library. Each action is called by `vellum x action_name`.

**Vellum Action** is just a simple executable that reads standard intermediary JSON from stdin (populated with album and config data) and performs some kind of logic based on this data. That's all. You can write actions in any language that supports reading JSONs (or even use simple bash scripts with jq) and use them to infinitely expand Vellum functionality in Unix Philosophy style. For developer provided actions and more context of what they may be useful for look into `actions/` directory.

## Main Arguments

#### `<name>`

Provides match to `[vl.actions].<name>` in `vellum.lua`. If `<name>` matches:

- Loads `[vl.config].environment` file
- Reads the `<name>.run` path of the executable
- Executes it with JSON palyload passed into its stdin

For cli ergonomics the `action_name` can be also called as `action-name` and it will still execute, because all of the `-` in name string will be replaced by `_` before it hitting the lua key.

#### `intermediary`

Built-in action used to print JSON payload directly into stdout. Useful for debug or development of new actions.

## Options

Options are used to specify which compiled albums (`album.lock.json` files) will be provided inside the JSON payload in form of `"albums": []` array. All options except `--` are mutually exclusive.

#### `--file` / `-f`

Provides an album by system path of either its `album.lock.json` or a directory containing this lock file. Default option, selected when no options given.

Argument: a path string. Defaults to `$(pwd)`.

Examples:

```bash
vellum x action -f '~/music/Path/To/Album/album.lock.json'
```

```bash
vellum x action -f '~/music/Path/To/Album/`
```

```bash
cd '~/music/Path/To/Album/'
vellum x action
```

#### `--playing` / `-p`

Provides currently queued (playing or paused) album. Boolean, has no arguments.

Examples:

```bash
vellum x -p action
```

```bash
vellum x action --playing
```

#### `--id`

Provides an album by its `id` in the collection. The `id` is the album's directory path relative to library root. It is always present in compiled `album.lock.json`.

Argument: a string value of `[album.lock.json].album.id` key.

Examples:

```bash
vellum x action --id 'Library/Relative/Path/To/Album/Directory'
```

#### `--query` / `-q`

Use this flag to select one or more albums from entire library using SQL that executes against in-memory SQLite database of the running server. Same `${}` shorthand expansion as in logic config can be used. The order passed from SQL is kept in `"albums": []` array.

Argument: an SQL string.

Examples:

```bash
# Selects albums with "total_tracks" > 20 and orders them by "date_added"
vellum x action -q '
WHERE ${album.total_tracks} > 20
ORDER BY ${album.date_added}
'
```

```bash
# Selects albums with "Brian Eno" in "albumartist" key and orders them by "original_date"
vellum x action -q '
${album.albumartist} LIKE %{Brian Eno}
ORDER BY ${album.info.original_date} DESC
'
```

#### `--`

Provides everything past it as options/arguments for the action itself.

## Specifications

### Intermediary JSON

The payload JSON contains three fields: `"albums"`, `"config"` and `"options"`

```jsonc
{
  // array populated with album.lock.json files
  "albums": [
    // 1st album.lock.json
    {
      "album": {
        // ...
      },
      "tracks": [
        // ...
      ]
    },
    // 2nd album.lock.json
    {
      // etc...
    }
    // etc...
  ],

  "config": {
    // `vl.config({})` table -> json
    "vellum": {},
    // `vl.actions({ <name> = { config = {} } })` table -> json
    "action": {},
  },

  // raw string passed after `--` option
  "options": ""
}
```

### Lua Config

For action to become executable it must be registered in `vellum.lua` config. 

```lua
vl.actions({

  action_name = {

    -- path to override expected executable action binary
    -- defaults to `~/.local/share/vellum/actions/{name}/{name}
    run = "",

    -- the config table
    -- can be populated with any static data
    -- converted to JSON and sent as a part of stdin payload
    config = {}, 
  }
})
```

### API

Each action can be executed from any interface by `/api/actions/{name}?{option}={argument}` endpoint.
