# Vellum

Vellum is an MPD client and album-centric library manager built on plaintext architecture for archivist-minded collectors.

## Philosophy

- **The Album as the Atomic Unit**. Vellum focuses purely on collection and management of albums. The point is to bring feeling of physical collecting to the digital landscape. Album is the base unit of Vellum because album is the base unit of any music collection in real life.
- **Immutable Audio / Mutable Metadata**. A ripped audio file should be a bit-perfect preservation of the original media. Audio files are inherently static; your metadata is inherently dynamic. This is why Vellum treats the audio file strictly as a read-only source and separates everything highly mutable into separate ancillary files. To configure album and tracks titles or provide genre you edit `metadata.toml`; to provide the cover you add `cover.jpg/png` next to it; to provide lyrics you create `lyrics/` directory and place text files in there. Plaintext metadata means your library can be controlled and versioned with Git. Every change to an artist's name or a custom tag can be tracked, reverted, backed up and synced to remote repository, completely independent of the audio files.

## How it works?

Think of an album folder the same way you think of a code repository. It contains the configuration file (the `metadata.toml` manifest) as well as the source files (audio files, cover art, lyrics, etc.). In this way album stops being an opaque object interpreted by media player and becomes a set of data points you can compile into json format. The engine reads your intent for an album expressed in the text manifest, scans the physical properties of the audio files (bit depth, duration), analyzes the artwork and links the lyrics. The result of this is the `metadata.lock.json` file. This is the file the server actually reads and uses to play the tracks and to register album in the collection.

## Getting Started

Vellum is in active development. To ensure a reproducible environment it is managed by Nix.

**Prerequisites:** 
* `nix`
* A running `mpd` instance

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

Once inside the Nix shell, install `node_modules` with bun:

```
cd web-app
bun install
```

And build the Rust binary:

```
vellum build
```

### 2. Configure Vellum

You create `.config/vellum/config.toml` file:
- In `[storage]` section you define `library_root = "path/to/your/library"` containing all of your album folders.
- Optionally in `[compiler.keys]` you define all tags besides standard ones you want to be present in `metadata.lock.json`. Format: `tag_name = { level = "album"/"track" }`. 

### 3. Configure Your Library

You place a folder containing album's audio files in your library root. To make it visible to Vellum you create `metadata.toml` file in it or run `vellum manifest` to read embedded tags and generate manifest from them. In this toml you have two sections: `[album]` header and multiple of `[[tracks]]` for each audio file. Tags are expressed in standard `TAGNAME = "Value"` format. The `[album]` header contains metadata *common* across an album (album artist, album title, genre, date, etc.), and each of `[[tracks]]` contains metadata *unique* to each track (track number, disc number, title).

Then you run `vellum update`. It automatically finds all new or changed `metadata.toml` files and compiles them with the source files into a `metadata.lock.json` artifacts.

### 4. Run the Stack

Because Vellum decouples the frontend UI from the backend coordinator, you will run them as separate processes:

```
# Terminal 1: Start the Rust backend
vellum server

# Terminal 2: Start the Svelte web interface
vellum ui
```

### CLI Usage

The `vellum` CLI tool is the central driver for managing your library's state. 

* `vellum manifest` — Scans your library root for unmanaged audio directories and generates the initial `metadata.toml` anchor files.
* `vellum update` — The core compiler command. Reads your TOML changes and writes the resolved `metadata.lock.json` files.
* `vellum server` — Starts the Axum backend server and the MPD synchronization watchdog.
* `vellum ui` — Starts the Vite/Svelte development server for the web interface.
* `vellum run <script>` — Executes Python automation scripts against the currently playing (or specified) album, such as fetching lyrics via Genius.
