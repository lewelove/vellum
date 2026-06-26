# Vellum

Vellum is an MPD client and album centric library manager built on plaintext architecture for archivist-minded collectors.

## Philosophy

- **The Album as the Atomic Unit**. Vellum focuses purely on collection and management of albums. The point is to bring feeling of physical collecting to the digital one. Album is the base unit of Vellum because album is the base unit of any music collection in real life.

- **Immutable Audio / Mutable Metadata**. A ripped audio files should be a bit-perfect preservation of the original media. Audio files are inherently static; your metadata is inherently dynamic. This is why Vellum treats the audio file strictly as a read-only source and separates everything mutable into separate ancillary files.

## Cool Features

### All Metadata in Plaintext
Entire library metadata: from songs names and album length in milliseconds, to custom album source URLs and ReplayGain values, to literally anything specific you can *write* about an album in your collection is stored and compiled within ancillary plaintext files. Edit them in Neovim, version control them, upload them to remote repository. Every change can be tracked, backed up and reverted, completely independent of the audio file's embedded tags. Once it's in Git you will never lose your library data ever again.

### Album as a Compiled Data Object
Think of an album directory as of an entry in the physical archive. It contains data written with human intent (`metadata.toml` and other `.toml` manifests) as well as the source files you're trying to preserve (audio, cover art, lyrics, etc...). In this way, album stops being an opaque fuzzy object interpreted by each different media player on the fly, and becomes a set of data points you can compile into a standard machine-readable JSON object (`album.lock.json`), the album's **index** in said archive. The engine reads your intent expressed in manifests, links the source files and scans the physical properties of the audio (bit depth, duration, etc...) to produce it. The `album.lock.json` is the file server uses to register album in your collection and to retrieve data from for further user-album interaction.

### Decoupled Frontend and Backend
**Vellum is a Rust web server first. User Interface intentionally comes second.** Want to change UI theme? Want to add some cool display feature? No worry. You can directly edit contents of the `web-app/` or completely rewrite your own UI in HTML, CSS and JavaScript, and run it in a browser, wiring it up to a running Vellum server using its web api **today**. Any UI framework supporting web api functionality can control MPD and retrieve album data through Vellum. You can build TUI apps, Godot based game-interfaces, or you can even use Curl to control it if you want to. Project's goal is to provide robust primitives, so you can interface your album collection in any weird+brilliant way.

## Contributing
I am the lone developer of this project. There are many desired, or even essential things not implemented currenly. If you have any ideas or requests, or if you want to contribute with patches, please submit everything to issues and PRs respectively. I would be sincirely happy to read through, work on and merge them. I am also open to code critique. I want to keep Vellum maintainable and open to new contributors. If you have constructive criticism of the codebase for me to take acton upon, please submit it to the issues as well.

## Getting Started

Vellum is in the state of active development. To ensure a reproducible environment it is managed by Nix.

**Prerequisites:** 
- `nix`
- A running `mpd` instance

### 1. Setup the Environment

Clone the repository:

```
git clone https://github.com/lewelove/vellum.git
cd vellum
```

Drop into the development shell:

```
nix develop
```

Or if you have `direnv` installed:

```
direnv allow
```

Once inside the Nix shell, install `node_modules` for UI with bun:

```
cd web-app
bun install
```

And build the Rust binary:

```
build --release
```

### 2. Configure Vellum

You create `~/.config/vellum/config.toml` file:
- In `[storage]` section you define `library_root = "path/to/your/library"` containing all of your album folders.
- Optionally in `[compiler.keys]` you define all tags besides standard ones you want to be present in `album.lock.json`. Format: `tag_name = { level = "album"/"track" }`. 

For config reference check out [my Vellum dotfiles](https://github.com/lewelove/nix-config/tree/main/dotfiles/.config/vellum). The config documetation is coming soon...

### 3. Configure Your Library

You place a folder containing album's audio files in your library root. To make it visible to Vellum you create `metadata.toml` file in it or run `vellum manifest` to read embedded tags and generate manifest from them. In this toml you have two sections: `[album]` header and multiple of `[[tracks]]` for each audio file. Tags are expressed in standard `tagname = "Value"` format. The `[album]` header contains metadata *common* across an album (album artist, album title, genre, date, etc.), and each of `[[tracks]]` contains metadata *unique* to each track (track number, disc number, title).

Then you run `vellum update`. It automatically finds all new or changed `metadata.toml` files and compiles them with the source files into a `album.lock.json` artifacts.

### 4. Run It

Because Vellum decouples the frontend UI from the backend coordinator, you will run them as separate processes:

```
# Terminal 1: Start the Rust backend
vellum server

# Terminal 2: Start the Svelte web interface
vellum ui
```

### CLI Usage

The `vellum` CLI tool is the central driver for managing your library's state. 

- `vellum manifest` — Scans your library root for unmanaged audio directories and generates the initial `metadata.toml` anchor files.
- `vellum update` — The core compiler command. Reads your TOML changes and writes the resolved `album.lock.json` files.
- `vellum server` — Starts the Axum backend server and the MPD synchronization watchdog.
- `vellum ui` — Starts the Vite/Svelte development server for the web interface.
- `vellum x` — Run defined actions via runtime router.
