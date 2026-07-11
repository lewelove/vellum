# vl.interfaces({ default = { config = {} } })

This document describes the default `web-app` interface config written in lua.

```lua
-- create lua table for handy color reuse down the line
-- populate it with CSS color strings
-- for the best human perceptual correctness the use of oklch() is recommended
local colors = {
  bright = "",
  oklch_200 = "",
  oklch_300 = "",
  oklch_400 = "",
  oklch_500 = "",
  -- etc...
}

-- same with CSS shadow strings
-- very handy indeed!
local shadows = {
  album_card = "",
  null = "",

}

vl.interfaces({

  default = {
    config = {

      -- all inside elemnts can be either an array of strings or a single string (will convert to array)
      -- the elements are injected into the top of `--font-stack` and `--font-mono`, order kept
      fonts = {
        -- used for main text drawn
        main = { "VellumInter" },
        -- used to draw tnum elements and strings of numbers (time, count index, etc...)
        monospace = { "VellumInterTnum" },
      },

        -- album grid in homeview
      album_grid = {
        spacing = { 
          -- two integers declaring horizontal and vertical distance between the album cards:
          x = 20,
          y = 16,
          -- integer declaring distance between top of window and first row
          top = 20,
        }
        -- album card in a grid
        album_card = {
          -- album card cover
          cover = {
            -- pixel size of the thumbnail in grid
            -- used in api cover fetch
            size = 200,
            -- one of cover vellum filters used inside api cover fetch as well
            filter = "lanczos",
          },
          -- text underneath the cover
          text = {
            -- render text for album card or not
            enable = true,
            -- top album title
            title = {
              size = 14,
              weight = 400,
              shadow = shadows.null,
            },
            -- bottom album artist
            albumartist = {
              size = 12,
              weight = 400,
              color = colors.muted
              shadow = shadows.null,
            },
            -- two integers describing vertical distance between:
            --   cover and title
            --   title and albumartist
            spacing = { 11, 2 },
          },
        },
      },
    },
  },
})
```
