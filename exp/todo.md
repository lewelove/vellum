# codebase

- Remove all hardcoded fallbacks from rust and create `vellum.lua` file to spec all of available values, and propagate them to rust from it
- Rewrite ALL default key resolution logic from rust to Lua
- Rewrite ALL type check systems in Lua
- Rename `vl.compiler.keys.album` and `vl.compiler.keys.tracks` -> `vl.compiler.album.keys` and `vl.compiler.tracks.keys`

# cli

### manifest

- Make `vellum manifest` a command you execute for each individual album specifically

### compile

- Add subcommand (`vellum compile album` / `vellum compile library`) that will compile either single album in pwd or all albums across library root
- Fix `track[].info.bitrate` kbps

# theming

- Make dropdown menus shadow same as panel shadow. Make them close on `esc`
- Add a way to render modal drawer at the home tab instantly, with zero animations. Tie it up to `open album` button in queue tab, so when pressed, the home tab is presented in view with drawer already fully open. ALso finally decire and remake their borders

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
