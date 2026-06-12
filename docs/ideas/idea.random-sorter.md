random sorter

the idea is to make new random sorter based on keys and seed that will deterministically display albums in specific order

```toml
[sorters.name]

label = "Seeded Random"

# `expression` can have only one attribute
# either `"sql string"` or `random`
# throw error and invalidate sorter otherwise
expression.random = {

  # if false -> invalidate the sorter
  # falls back to true
  enable = true

  # any string
  # falls back to empty string
  seed = "seed"

  # keys from album.lock.json that will be used to calculate hash
  # falls back to album.id
  keys = [ "album.album", "album.albumartist", ... ]
}
```

how it works:

- rust takes the `keys` values and `seed` and calculates the 256 bit BLAKE3 hash of all of them
- orders albums by hash produced using radix sort algorithm

this way with given `keys` and `seed` albums order in a view is guaranteed to be deterministic
