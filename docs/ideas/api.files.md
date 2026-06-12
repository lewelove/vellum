Create generic `/api/files/{name}`. Rust reads the `files.{name}` path value in config.toml -> serves the file to ui with MIME type resolved using mime_guess crate

Rules:
- All paths in `[files]` must be canonicalized by rust to strictly lie within `~/.config/vellum/` folder

Attributes:
```toml
# config.toml

[files]

name = "files/file.txt"

```


