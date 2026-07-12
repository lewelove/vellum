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
  drawer: {
    "drawer-padding-y": 18,
    "drawer-padding-x": 18,
    "drawer-font-size-album": 21,
    "drawer-font-size-artist": 18,
    "drawer-font-size-track": 14,
    "drawer-track-y": 32,
    "drawer-split-gap": 24,
    "drawer-contents-x-max": 1600
  },
  album_grid: {
    spacing: { x: 20, y: 16, top: 20 },
    album_card: {
      cover: { size: 200, filter: "lanczos" },
      text: {
        enable: true,
        title: { size: 14 },
        albumartist: { size: 12 },
        spacing: { top: 11, middle: 2 },
      }
    }
  }
});

export function updateTheme(config: any) {
  if (!config) return;
  if (config.theme) {
    if (config.theme.palette) Object.assign(theme.palette, config.theme.palette);
    if (config.theme.colors) Object.assign(theme.colors, config.theme.colors);
    if (config.theme.drawer) Object.assign(theme.drawer, config.theme.drawer);
  }
  if (config.album_grid) {
    if (config.album_grid.spacing) Object.assign(theme.album_grid.spacing, config.album_grid.spacing);
    if (config.album_grid.album_card) {
      if (config.album_grid.album_card.cover) Object.assign(theme.album_grid.album_card.cover, config.album_grid.album_card.cover);
      if (config.album_grid.album_card.text) {
        if (config.album_grid.album_card.text.title) Object.assign(theme.album_grid.album_card.text.title, config.album_grid.album_card.text.title);
        if (config.album_grid.album_card.text.albumartist) Object.assign(theme.album_grid.album_card.text.albumartist, config.album_grid.album_card.text.albumartist);
        if (config.album_grid.album_card.text.spacing) Object.assign(theme.album_grid.album_card.text.spacing, config.album_grid.album_card.text.spacing);
        if (config.album_grid.album_card.text.enable !== undefined) theme.album_grid.album_card.text.enable = config.album_grid.album_card.text.enable;
      }
    }
  }
}
