# `vellum run`

## ARGUMENTS

has 1 argument
- `vellum run <NAME>`

`<NAME>`
- provides match to `scripts.<NAME>` in `config.toml`

if `<NAME>` matches:
- reads path string value from it
- canonicalize it to exist strictly in `~/.config/vellum`
- executes it with `json` (based on flag) passed into its stdin

## FLAGS

flags are used to provide requsted album `id` and select json to pass
at least one flag must be provided

### `--playing` / `-p`

passes currently playing album json directly

### `--id`

has 1 argument
- `--id <ID>`

`<ID>`
- string of any album's id in collection

if `<ID>` found:
- selects json
- passes json selected into script stdin

### `--intermediary` / `-i`

halts execution and prints generated json into stdout
