Create generic `/api/scripts/{name}` that is sent to rust server with json in HTTP request body. Rust reads the `commands.{name}.path` value in config.toml -> passes the json received by ui to executable stdin -> executes it.

Security:
- The client's physical IP must ALWAYS be the loopback interface (127.0.0.1 or ::1). Reject any other IP immediately.
- If `Origin` HTTP header is present -> allow only if point to `127.0.0.1` or `localhost`

Rules:
- All paths in `[scripts]` must be canonicalized by rust to strictly lie within `~/.config/vellum/` folder

Attributes:
```toml
# config.toml

[scripts]

# name of the script that will be used in api call
# you must `chmod +x` this file
name = "scripts/some_script.sh"

```


