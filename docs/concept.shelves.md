# Shelves Concept

Shelves are the frozen subsets of all albums in collection, configured by filter / text file and default sorter.

Think of shelf as of a literal shelf in physical album cabinet, with specific albums on it in specific order.

## Shelves Attributes

```toml

[shelves.name]

# path to .txt file containing the albums `id` strings separated by \n
file = "shelves/my_shelf.txt"

# the SQL filter expression
# if `file` is present -> applies filter to given ids in file
# if `file` is omitted -> applies filter to entire library
filter = ""

# define all sorters used for this shelf
sorters = []

```
