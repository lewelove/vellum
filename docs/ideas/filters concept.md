filters concept

the idea is to add new `[filters]` generic in logic.toml

it will be displayed as new top element in home sidebar

the `[filters.name]` will contain the `expression = ""` with sql filter string and `label = ""` to display it in ui

the `[libraries.name]` will now contain the `filters = [ "name", ... ]` array with all `filters.name` matched

if `libraries.name.filters` is array and contain only one element (or is string) the dropdown menu of filters is not displayed in ui

we also rename `select` in `groupers` AND `order_by` in `sorters` -> `expression`
