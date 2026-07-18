# vellum x collect

The `collect` is an action used to add album to the collection without the actual audio files. This action consumes the album source url as primary argument.

Url of the album must be one of the services such as:

- Discogs
- Musicbrainz

## How it works

Upon execution `vellum x collect`:

- Determines the service used from the url from `{stdin_json}.options[]`
- Selects API key for determined service from environment loaded by Vellum and fetches album data
- Creates the `{config.root}/{config.formatting.album}`
- Saves the raw data returned from the service inside `{config.root}/{config.formatting.album}/{config.formatting.info}/{service_name}_{type}.json`

For `collect` action we target the `discogs.com/master` albums, for we don't know which specific `release` we can find and acquire later. The main argument can be one of 3 URLs:

If `https://www.discogs.com/master/`:

- Save `discogs_master.json`

If `https://www.discogs.com/release/`:

- Resolve `discogs.com/master` URL from it
- Proceed with `discogs.com/master` logic

If `https://musicbrainz.org/release`

- Save `musicbrainz_release.json`
- Resolve `discogs.com/master` and `musicbrainz.org/release-group` URL from it
- Proceed with `discogs.com/master` logic

All prepending `https://` or `https://www.` can be omitted, it will still work

> If URL was the `musicbrainz.org/release` type and `discogs.com/master` cannot be resolved we roll with data from it, because `musicbrainz.org/release-group` has no actual album data.

Then inside album directory:

- Write `metadata.toml` and `local.toml`
- For each `discogs.com/master` and `musicbrainz.org/release` source save its cover under `Digital Covers/{service_name}.png`
- For `discogs.com/master` source save cover it under `cover.png`

## Specifications

### Lua Config

```lua
vl.actions({ collect = {
  config = {
    -- root directory of where album directory will be created
    -- required, cannot be ran without it
    root = "",
    -- all "{key}" will be regex replaced by actual strings
    formatting = {
      -- album directory
      album = "{albumartist} - {album}"
      -- info directory for .json files
      info = "Info"
    }
  }
}})
```

### `metadata.toml` 

Use the `discogs_master.json` data, with fallback to `musicbrainz_release.json`

```toml
[album]

albumartist = ""
album = ""
date = ""

# always from Discogs 
genre = [ "", "", ... ]
styles = [ "", "", ... ]

# for each resolved url based on link from primary argument
# omit if cannot resolve

# always a "discogs.com/master"
discogs_url = "https://discogs.com/master/..."
# always a "musicbrainz.org/release-group"
musicbrainz_url = "https://musicbrainz.org/release-group/..."

# repeat for each track in data fetched
[[tracks]]
# omit if number of discs < 2
discnumber = 1
tracknumber = 1
title = ""
# omit if same as albumartist
artist = ""
# this flag will prevent the compiler error due to yet non-existent track file
path = false
```

### `local.toml`

```toml
[local]

# ISO format at UTC+0 with milliseconds of system datetime upon `vellum x collect` execution
date_added = 1999-12-31T00:00:00.000Z
```
