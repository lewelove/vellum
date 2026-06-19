# Library Concept

Library is a set containing `filters`, `groupers` and `orders`, which can be applied one to another, allowing creating dynamic ui views with high combinatorial complexity.

# Library Attributes

```toml

[libraries.name]

label = "Library Name"

include = {

  # define all filters that can applied to entire album pool
  # if `array`
  #   - select [filters] by name
  # if `true`:
  #   - all filters except `private` included
  # in other cases:
  #   - no filters included
  #   - no ui menu displayed
  #   - entire album pool is used
  filters = [],

  # define all groupers to group filtered albums
  # if `array`
  #   - select [groupers] by name
  # if `true`:
  #   - all filters except `private` included
  # in other cases:
  #   - library invalidated & excluded from ui
  groupers = [],
  
  # define all orders used for this cabinet
  # if `array`
  #   - select [orders] by name
  # if `true`:
  #   - all orders except `private` included
  # in other cases:
  #   - no orders included
  #   - no ui menu displayed
  #   - alphabetic order by `album.id` is used
  orders = [],

}

exclude = {
  # for each attribute
  # if `array`
  #   - exclude from `include.filters`
  # if `true`
  #   - exclude all
  # in other cases
  #   - no effect
  # defaults to `false`
  filters = [],
  groupers = [],
  orders = [],
  
}

```
