## vl.actions

This function provides the data for `vellum x` to run

```lua

vl.actions({

  action_name = {

    -- path to override expected executable action binary
    -- defaults to `~/.local/share/vellum/actions/{name}/bin.sh`
    run = "",

    -- the config table
    -- can be populated with any static data
    -- converted to JSON and sent as a part of stdin payload
    config = {}, 
  }

})
```
