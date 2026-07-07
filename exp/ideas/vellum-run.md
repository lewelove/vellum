# `vellum run`

## Arguments

has 1 argument
- `vellum run <NAME>`

`<NAME>`
- provides match to `scripts.<NAME>` in `config.toml`

if `<NAME>` matches:
- reads path string value from it
- executes it with `json` (based on flag) passed into its stdin

## Flags

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

### Intermediary Form

the json passed to scripts stdin must contain an array of albums in order returned by sql + config definition

```json
{
  [
    {
      "album": {
        // ...
      }
    },
    {
      // etc...
    }
  ],

  "config": {}
}
```

