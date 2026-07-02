import { collection } from "./collection.svelte.ts";
import { sync } from "./sync.svelte.ts";

class Prewarmer {
  pinnedTextures: Map<string, ImageBitmap> = $state(new Map());

  constructor() {
    sync.addEventListener('message', (e: Event) => {
      const json = (e as CustomEvent).detail;
      if (json.type === "INIT_DICT" || json.type === "ALBUM_UPDATED" || json.type === "CONFIG_UPDATE" || json.type === "INTERFACE_CONFIG_UPDATE") {
        this.orchestrate();
      }
    });
  }

  async orchestrate() {
    const concurrencyLimit = 6;
    const queue = Object.values(collection.dict);
    let pendingUpdates = false;
    let lastFlush = Date.now();

    const flush = () => {
      this.pinnedTextures = new Map(this.pinnedTextures);
      pendingUpdates = false;
      lastFlush = Date.now();
    };

    const processor = async () => {
      while (queue.length > 0) {
        const album = queue.shift();
        const url = collection.getThumbnailUrl(album);
        if (!url || this.pinnedTextures.has(url)) continue;
        try {
          const res = await fetch(url);
          const blob = await res.blob();
          const bitmap = await createImageBitmap(blob, {
            premultiplyAlpha: 'none',
            colorSpaceConversion: 'default'
          });
          this.pinnedTextures.set(url, bitmap);
          pendingUpdates = true;
          if (Date.now() - lastFlush > 100) flush();
        } catch (err) {}
      }
      if (pendingUpdates) flush();
    };

    const workers = Array.from({ length: concurrencyLimit }, () => processor());
    await Promise.all(workers);
    if (pendingUpdates) flush();
  }
}

export const prewarmer = new Prewarmer();
