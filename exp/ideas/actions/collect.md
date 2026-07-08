# vellum x collect

The `collect` is an action used to add album to the collection without the actual audio files. This action consumes the album source url as primary argument.

Url of the album must be one of the services such as:

- Discogs
- Musicbrainz
- Bandcamp
- YouTube

## How it works

Upon execution `vellum x collect`:

- Determines the service used from the url
- Selects API key (if required) for determined service from `environment` file
- Fetches album data
- Creates the album directory inside the target one using `{albumartist} - {album}` pattern (can be configured)

Then inside this directory:

- Saves the raw data returned from the service inside `Info/{service_name}.json`
- If cover can be fetched it saves it under `cover.png`
- Registers the `date_added` (moment the action is executed) + `enabled = false` keys inside `local.toml`
- Creates the `metadata.toml` file populated with basic album data
- Creates `{service_name}.toml` with only `[album]` populated with single `{service_name}_url = "{url}"`
