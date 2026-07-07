# vl.compiler.covers

## Specifications

### How cover is processed:

For each `cover_name`:

- The `source` and `required` are resolved into image file path
- Image file hash is calculated
- Address hash is calculated by file hash + `size` + `filter` + `ratio` values
- Master image is generated using `size`, `filter` and `ratio` and saved as `~/.cache/vellum/covers/{cover_name}/master/{address_hash}.qoi`
- If `cache` -> for each `cache[idx]` the cache image is generated via the resize of the master image using `cache[idx].filter` and `cache[idx].size` and saved as `~/.cache/vellum/covers/{cover_name}/static/{filter}/{size}/{address_hash}.qoi`
- Cover data is linked into the `[album.lock.json].album.covers` using this json specification:

```json
{
  "cover_name": {

    "address": "",    // address hash
    "size": 1,        // cover_name.size
    "filter": "",     // cover_name.filter
    "ratio": "",      // cover_name.ratio

    "file": {
      "path": "",     // source image file path
      "hash": "",     // source image file hash
      "mtime": 1,     // source image mtime
      "byte_size": 1  // source image byte size
    }
  }
}
```

We generate master image in the first place to always have the source on fastest system disk for the `/api/covers/` dynamic generation and serving.

### Source

The `source` defines the path of the image relative to metadata.toml that will be targeted by the compiler. Can be either:

- `"path/to/image.ext"`: A path string. Targets the single image directly.
- `{ "path/1.ext", "path/2.ext", ... }`: An array of path strings. Targets the first valid image in it.

Is required.

### Required

The `required` defines the requirement of `source` target existence. A boolean:

- `true`: If none of `source` paths are valid images -> throw compile error.
- `false`: If none of `source` paths are valid images -> skip the cover compilation and linkage to lock.

Not required. Defaults to `false`.

### Filter

The `filter` defines the interpolation algorithm used to resize the `source` image into master cover, or master into `cache`. A string that maps to `fast_image_resize` Rust crate. Can be either:

- `"box"`: maps to `FilterType::Box`
- `"bilinear"`: maps to `FilterType::Bilinear`
- `"hamming"`: maps to `FilterType::Hamming`
- `"catmullrom"`: maps to `FilterType::CatmullRom`
- `"mitchell"`: maps to `FilterType::Mitchell`
- `"gaussian"`: maps to `FilterType::Gaussian`
- `"lanczos"`: maps to `FilterType::Lanczos3`

Is required.

### Size

The `size` defines the pixel size of the square the resulting image must fit in. In other words the pixel size of the largest side. Must be a positive integer.

### Ratio

The `ratio` defines the the dimentions ratio of the center crop for compiled master image. A string, can be either:

- 2 positive integers separated by `:` (e.g. `"1:1"` or `"16:9"`): Integers are separated and used as ratio.
- `"original"`: Skips the cropping.

Not required, defaults to `"1:1"`.

### Cache

The `cache` is used to define the parameters of covers generated from master and cached at compile time. Their existence is recommended for the reason of the instant disk retrival (cahce hit) when `/api/covers/{cover_name}/{filter}/{size}/{address}` is called. If you know under which filter and dimensions your user interface calls the `/api/covers` most oftenly, you can provide this data here and make all future calls just a cache read.

The `cache` is an array of 2 element arrays containing:

- `filter`
- `size`

No exactly equal arrays must exist in `cache`. If found throw config error.

Not required. Default to nothting.

### Lua specification:

```lua
vl.compiler.covers({

  cover_name = {

    source = { "" },

    filter = "",

    size = 1,

    ratio = "",

    cache = {
      { filter = "", size = 1 },
      { filter = "", size = 1 },
      -- etc...
    },
  },
})
```

### Default config:

```lua
vl.compiler.covers({

  main = {

    source = {
      "cover.png",
      "cover.jpg",
      "cover.jpeg",
      "front.png",
      "front.jpg",
      "front.jpeg",
      "folder.png",
      "folder.jpg",
      "folder.jpeg",
    },

    required = true,

    filter = "mitchell",
    size = 1080,
    ratio = "1:1"

    cache = { { filter = "lanczos", size = 200 } },
  },
})
```
