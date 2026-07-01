export const theme = $state({
  palette: {
    100: "#242424",
    200: "#323232",
    300: "#424242",
    400: "#CCCCCC",
    500: "#FFFFFF",
  },

  colors: {
    "background-main": "200",
    "background-drawer": "100",
    "text-main": "500",
    "text-muted": "400",
    "border-muted": "300",
  },

  typography: {
    "font-size-title": 14,
    "font-weight-title": 400,
    "font-size-artist": 12,
    "font-weight-artist": 400,
  },

  albumGrid: {
    "crease-height": 20,
    "gap-x": 20,
    "gap-y": 16,
    "cover-size": 200,
    "text-gap-main": 11,
    "text-gap-lesser": 2,
    "font-line-height-title": 16,
    "font-line-height-artist": 14,
    "drawer-gap-main": 0,
    "drawer-chevron-height": 12,
    "drawer-chevron-width": 24,
  },

  drawer: {
    "drawer-padding-y": 18,
    "drawer-padding-x": 18,
    "drawer-font-size-album": 21,
    "drawer-font-size-artist": 18,
    "drawer-font-size-track": 14,
    "drawer-track-y": 32,
    "drawer-split-gap": 24,
    "drawer-contents-x-max": 1600
  }
});

export function updateTheme(config: any) {
  if (!config || !config.theme) return;
  if (config.theme.palette) Object.assign(theme.palette, config.theme.palette);
  if (config.theme.colors) Object.assign(theme.colors, config.theme.colors);
  if (config.theme.typography) Object.assign(theme.typography, config.theme.typography);
  if (config.theme.albumGrid) Object.assign(theme.albumGrid, config.theme.albumGrid);
  if (config.theme.drawer) Object.assign(theme.drawer, config.theme.drawer);
}
