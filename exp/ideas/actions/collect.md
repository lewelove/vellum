# vellum x collect

The moment you decide the album *must* exist in your collection is much more important the the moment you physically acquire it through the digital means, though both moments must be recorded. And the first moment should be historical one, because it reflects the time and the state you were in when you acted upon it. To draw comparison to means of "collecting" (quotes used because you never own it) albums in streaming services: the act of deciding and the act of adding is never separated. It's just a click. A streaming platform serves you the feeling of infinite freedom and abundance, you can just commit to the adding spree whenever you want, without having to do the work of acquiring the albums themselves. That's a real feeling, and the feeling i've been longing for without recognizing it for all the years i separated myself from streaming.

The `collect` is an action used to add album to the collection without the actual audio files. This action consumes the album source url as primary argument.

Url of the album must be one of the services such as:

- Discogs
- Musicbrainz

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
