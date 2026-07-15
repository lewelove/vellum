# Vellum

Vellum is an MPD client and album centric library manager built from first Unix Philosophy principles for archivist-minded collectors.

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

### Vellum Actions
Since every album is compiled into a plaintext JSON, every album becomes scriptable. **Vellum Action** is a simple standalone executable that reads standard intermediary JSON from stdin (generated and populated with albums and config data at runtime) and performs some kind of logic based on this data. That's all. You can write actions in any language that supports reading JSONs (or even use simple bash scripts with jq) and use them to infinitely expand Vellum functionality in Unix Philosophy style. You can make each action configurable via its own cli arguments and `vellum.lua`, as Vellum provides the entire pipeline. Everything: from *deciding* to add the album to your collection (yes even time of decision should be recorded), to searching for & acquiring it on torrent trackers or soulseek, to cover search and metadata fetch and eventual compilation and playback, to all of the future cool logic you can conjure up from raw album data, can be managed via Vellum Actions. For built-in actions and more context of what they may be useful for look into `actions/` directory.

## Contributing
I am the lone developer of this project. There are many desired, or even essential things not implemented currently. If you have any ideas or requests, or if you want to contribute with patches, please submit everything to issues and PRs respectively. I would be sincerely happy to read through, work on and merge them. I am also open to code critique. I want to keep Vellum maintainable and open to new contributors. If you have constructive criticism of the codebase for me to take action upon, please submit it to the issues as well.

## AI Disclosure & Human Design
I use LLMs for research and *writing* out the code. I use them for nothing more. Everything besides raw syntax implementation is intentionally human-designed by none other than me. I do not rely on agents for coding, keeping the good old "chat -> file -> diff" workflow, as I will **never** allow the clanker to touch my filesystem. I design the system architecture, specifications, logic, in other words how *everything* must work and feel. I review every file diff and often reject the code until i'm satisfied with the quality. All of the documentation and this README you are reading, are handrolled on my cheap old keyboard from my bedroom. **I am the primary (single, ha-ha) and the most active user of Vellum.** This project was born from passionate love of album collecting, respect for Unix Philosophy, and genuine lack of anything like it. I want Vellum to be free and open source forever, thus the AGPL-3.0 license. I'll try to commit maintaining it as long I will be using Linux, as well as collecting and listening to albums, which gives us, according to average life expectancy, around 50+ years of time. Thank you for reading this README.md and (hopefully) using this software. All feedback is always appreciated.

## Getting Started

**Prerequisites:** 

- Nix
- Bun
- An active MPD instance

Vellum is in the state of active development. To ensure a reproducible environment it is managed by a Nix Flake. All further setup will be based on Nix prerequisite. All of the setup can be achieved without Nix, simply by having Cargo & Bun in shell, just not reproducibly.

### 1. Setup the Environment

Clone the repository:

```bash
git clone https://github.com/lewelove/vellum.git
cd vellum
```

Drop into the development shell:

```bash
nix develop
```

Or if you have `direnv` installed:

```bash
direnv allow
```

For default interface to run from cloned developer repo you must `cd` into its directory, install `node_modules` and `chmod +x run.sh`:

```bash
cd interfaces/web-app
bun install
chmod +x run.sh
```

And build the Rust binary, as well as all of the Vellum Actions:

```bash
build
```

The `vellum` executable will be located at `./rust/target/release/vellum`. Alias this path in your shell of choice for further use.

### 2. Configure Vellum

You create `~/.config/vellum/vellum.lua` file:

```lua
-- Since we are using Lua, a truly god-tier config language, for convenience you can define cloned repository path as local string variable
local repo_dir = "Path/To/Cloned/Repo/"

vl.config({ storage = { 
  -- Define a library path containing all your albums
  library = "Path/To/Your/Library/Root/"
}})

-- Optionally you can define all keys besides standard ones you want to load from toml manifests and be present in `album.lock.json`

-- [album.lock.json].album.keys level
vl.compiler.keys.album({
  album_key_name = true
})

-- [album.lock.json].tracks[].keys level
vl.compiler.keys.tracks({ 
  track_key_name = true
})

-- For `vellum interface` command to run you point default interface to `interfaces/web-app` directory from the previous step
vl.interfaces({ default = {
  -- Quite handy!
  directory = repo_dir .. "interfaces/web-app/"
}})
```

For config reference check out [my Vellum dotfiles](https://github.com/lewelove/nix-config/tree/main/dotfiles/.config/vellum). The config documentation is coming soon...

### 3. Configure Your Library

You place a folder containing album's audio files in your library root. To make it visible to Vellum you create `metadata.toml` file in it or run `vellum manifest` to read embedded tags and generate manifest from them. In this toml you have two sections: `[album]` header and multiple of `[[tracks]]` for each audio file. Tags are expressed in standard `keyname = "Value"` format. The `[album]` header contains metadata *common* across an album (album artist, album title, genre, date, etc.), and each of `[[tracks]]` contains metadata *unique* to each track (track number, disc number, title).

Then you run `vellum update`. It automatically finds all new or changed `metadata.toml` files and compiles them with the source files into a `album.lock.json` artifacts.

### 4. Run It

Because Vellum decouples the interface from the backend server, you will run them as separate processes:

```bash
# Terminal 1: Start the Rust backend
vellum server
```

```bash
# Terminal 2: Start the default interface (Svelte Web App)
vellum interface
```

## CLI Usage

The `vellum` CLI tool is the central driver for managing your library's state. 

- `vellum manifest` — Scan your library root for unmanaged audio directories and generate the initial `metadata.toml` manifest.
- `vellum update` — The core compiler command. Reads your TOML manifests and compiles the `album.lock.json` files.
- `vellum server` — Start the Axum backend server.
- `vellum interface` — Run interfaces defined in `vl.interfaces`.
- `vellum x` — Run defined actions via runtime `vl.actions` router.
