# codebase

- Remove all hardcoded fallbacks from rust and create `vellum.lua` file to spec all of available values, and propagate them to rust from it
- Rewrite ALL default key resolution logic from rust to Lua
- Add album hot-reload upon each registered manifest mtime change
- Register manifests like `name` and directory paths as `directory.name`


# cli

### manifest

- Make `vellum manifest` a command you execute for each individual album specifically

### compile

- Add subcommand (`vellum compile album` / `vellum compile library`) that will compile either single album in pwd or all albums across library root
- Add `--force` flag

### update

- Reject target album if it's not in `storage.library`

# theming

- Make absolutely all white elements derived from single `oklab` white value using alpha channel

# api

- Add /mpd prefix to all mpd control related api endpoint. Example: /api/mpd/play_album

# actions

- Remove any kind of terminal messages printing unless `--debug` is used
- Make `vellum x` drop into process output just like the `vellum interface` does

### cover-palette

- Add little run.sh script that executes actual binary and then opens the file generated

### open-album-directory-in-terminal

- Built-in action
- Bash script that spawns terminal with `cd {[config].storage.library}/{album.id}`

### open-album-directory-in-file-manager

- Built-in action
- Bash script that spawns terminal with `cd {[config].storage.library}/{album.id}`

# config

### logic

- Rename the `strict` keyword to `private` in `logic.toml`
- Provide `include` / `exclude` keywords for all arrays of orders, groupers and filters
