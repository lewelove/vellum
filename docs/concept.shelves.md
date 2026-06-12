# Shelves Concept

Shelves are the frozen subsets of all albums in collection, configured by filter / text file and default sorter.

Think of shelf as of a literal shelf in physical album cabinet, with specific albums on it in specific order.

## Shelves Attributes

```toml

[shelves.name]

# path to .txt file containing the albums `id` strings separated by \n
file = "shelves/my_shelf.txt"

# the SQL filter expression
# if `file` is valid:
#   - applies filter to given ids in file
# in other cases:
#   - applies filter to entire library
filter = ""

# the SQL order expression
# if invalid & `file` is valid:
#   - uses original file id order
# if valid & `file` is invalid:
#   - uses alphabetic sort of album.id
# in other cases:
#   - orders the filter result
order = ""

```
