# Cabinet Concept

Cabinet is a set of static `shelves` and set of `sorters` to apply on top

## Cabinet Attributes

```toml

[cabinets.name]

label = "Cabinet Name"

# define all shelves in cabinet by their name
# if `array`:
#   - select [shelves] by name
# if `true`:
#   - all shelves except `private` are included
# in other cases:
#   - cabinet invalidated & excluded from ui
shelves = []

# define all sorters used for this cabinet
# if `array`
#   - select [sorters] by name
# if `true`:
#   - all sorters except `private` included + "Original" (original shelf order)
# in other cases:
#   - no sorters included
#   - no ui menu displayed
#   - shelves original order is used
sorters = []

```

