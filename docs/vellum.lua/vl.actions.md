## vl.actions

This function regiters an action and provides the data for `vellum x` to run it.

### Specifications

#### Execution:

Each action is executed by `vellum x action_name`. For cli ergonomics the `action_name` can be also called as `action-name` and it will still execute, because all of the `-` in name string will be replaced by `_` before it hitting the lua key.

#### Action specification:

```lua
vl.actions({

  action_name = {

    -- path to override expected executable action binary
    -- defaults to `~/.local/share/vellum/actions/{name}/{name}
    run = "",

    -- the config table
    -- can be populated with any static data
    -- converted to JSON and sent as a part of stdin payload
    config = {}, 
  }
})
```
