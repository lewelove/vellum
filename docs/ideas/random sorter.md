random sorter

the idea is to make new random sorter based on keys and seed that will always display albums in specific order if all function keys are equal

```toml
[sorters.name]

label = "Seeded Random"

# `expression` can have only one attribute
# either `"sql string"` or `random`
# throw error and invalidate sorter otherwise
expression.random = {

  # falls back to true
  # if false -> invalidate the sorter
  enable = true

  # any string
  # falls back to system time at evaluation time
  seed = "seed"

  # keys from album.lock.json that will be used to calculate hash
  keys = [ "album.album", "album.albumartist", ... ]
}
```

how it works:

- rust takes the `keys` values and `seed` and calculates the 256 bit BLAKE3 hash of all of them
- orders albums by hash produced using radix sort algorithm

this way with given `keys` and `seed` albums order in a view is guaranteed to be deterministic
