# Library Concept

Library is a set containing `filters`, `groupers` and `sorters`, which can be applied one to another, allowing dynamic ui views with high combinatorial complexity.

# Library Attributes

```toml

[libraries.name]

label = "Library Name"

# define all filters that can applied to entire album pool
# if `array`
#   - select [filters] by name
# if `true`:
#   - all filters except `private` included
# in other cases:
#   - no filters included
#   - no ui menu displayed
#   - entire album pool is used
filters = []

# define all groupers to group filtered albums
# if `array`
#   - select [groupers] by name
# if `true`:
#   - all filters except `private` included
# in other cases:
#   - library invalidated & excluded from ui
groupers = []

# define all sorters used for this cabinet
# if `array`
#   - select [sorters] by name
# if `true`:
#   - all sorters except `private` included
# in other cases:
#   - no sorters included
#   - no ui menu displayed
#   - alphabetic order by `album.id` is used
sorters = []

```
