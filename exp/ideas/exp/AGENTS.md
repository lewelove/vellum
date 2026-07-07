# AGENTS.md

## Rust Code Structure

Rust code for `vellum` binary (in `./rust/vellum`) follows strict pattern:

- For CLI commands and its subcommands use `{command}/{subcommand}/../mod.rs`
- For commands and subcommands modules (to split logic into smaller self-documenting files) use `{command}/{subcommand}/../{self_describing_module_name}.rs`

## Rust Code Style and Formatting

- Keep new files added under 300 LoC
- Never change `log::info!`, `log::debug!`, `log::warn!` and `log::error!` messages willy-nilly, only when logic change actually requires it or when explicitly asked to
- For every file printed use standard professional `crate.io` style comment blocks

## Clippy Rules to Always Satisfy

When outputting code make sure you always satisfy these clippy rules, so i don't have to paste the compile errors back to you

- `clippy::collapsible-if`
- `clippy::uninlined-format-args`
- `clippy::option_if_let_else`
- ``

