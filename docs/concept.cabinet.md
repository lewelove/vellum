# Cabinet Concept

Cabinet is a set of static `shelves` and set of `orders` to apply on top

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

# define all orders used for this cabinet
# if `array`
#   - select [orders] by name
# if `true`:
#   - all orders except `private` included + "Original" (original shelf order)
# in other cases:
#   - no orders included
#   - no ui menu displayed
#   - shelves original order is used
orders = []

```

