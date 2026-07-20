# system.toml

This file is generated alongside the `metadata.toml` at manifest time and looks like this:

```toml
[album.system]

date_generated = # {now_in_iso_format} #
```

If `system.toml` cannot be found next to `metadata.toml` at compile time it is generated as well.
