# Cabinet Concept

Cabinet is a set of `shelves` and `sorters`

## Cabinet Attributes

```toml

[cabinets.name]

# define all shelves in cabinet by their name
shelves = []

# define all sorters used for this cabinet
# if `sorters = true` -> all sorters except `strict` are added
# all cabinets always contain one "default" sorter
#   for shelf defined with `file` it's an original order of ids in text
#   for shelves defined without `file` it's a `sorters.default` or `id`
sorters = []

```

