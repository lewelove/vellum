# vellum.lua

This file describes and outlines future documentation of vellum configuration written in lua.

---

The `~/.config/vellum/vellum.lua` is the initial file that must exist for config to be active.

## require()

For modularizing config use `require("name")` of the `name.lua` files reative to `~/.config/vellum/`. For path specification use `.` as the delimiter.

```lua

-- imports ~/.config/vellum/module.lua
require("module")

-- imports ~/.config/vellum/folder/module.lua
require("folder.module")
```
