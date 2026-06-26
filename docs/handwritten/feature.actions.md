# Vellum Actions

Actions allow you to interface your collection via `vellum x` CLI command. If you provide executable file path under `[config.toml].actions.{name}`, the `vellum x {name}` will run it with JSON string injected into its stdin.

The injected json will have 2 elements:
- Array of `album.lock.json` files, selected via `--id '{album id string}'` or `--query '{SQL query string}'`
- The `config.toml` converted into json

You can print entire string into stdout by providing `--intermediary` flag
