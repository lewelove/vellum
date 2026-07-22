import { collection } from "./collection.svelte.ts";
import { sync } from "./sync.svelte.ts";

class Prewarmer {
  pinnedTextures: Map<string, ImageBitmap> = $state(new Map());
  private inFlight = new Set<string>();
  private failedUrls = new Set<string>();
  private pendingUpdates = false;
  private lastFlush = Date.now();
  private flushTimer: number | null = null;

  constructor() {
    sync.addEventListener('message', (e: Event) => {
      const json = (e as CustomEvent).detail;
      if (json.type === "INIT_DICT" || json.type === "ALBUM_UPDATED" || json.type === "CONFIG_UPDATE" || json.type === "INTERFACE_CONFIG_UPDATE") {
        this.orchestrate();
      }
    });
  }

  flush() {
    if (!this.pendingUpdates) return;
    this.pinnedTextures = new Map(this.pinnedTextures);
    this.pendingUpdates = false;
    this.lastFlush = Date.now();
    if (this.flushTimer) {
      clearTimeout(this.flushTimer);
      this.flushTimer = null;
    }
  }

  scheduleFlush() {
    this.pendingUpdates = true;
    if (Date.now() - this.lastFlush > 50) {
      this.flush();
    } else if (!this.flushTimer) {
      this.flushTimer = setTimeout(() => this.flush(), 50) as any;
    }
  }

  async loadNow(url: string) {
    if (!url || this.pinnedTextures.has(url) || this.inFlight.has(url) || this.failedUrls.has(url)) return;
    
    this.inFlight.add(url);
    try {
      const res = await fetch(url);
      if (!res.ok) throw new Error("Failed to load");
      const blob = await res.blob();
      const bitmap = await createImageBitmap(blob, {
        premultiplyAlpha: 'none',
        colorSpaceConversion: 'default'
      });
      this.pinnedTextures.set(url, bitmap);
      this.scheduleFlush();
    } catch (err) {
      this.failedUrls.add(url);
    }
    this.inFlight.delete(url);
  }

  async orchestrate() {
    const concurrencyLimit = 6;
    const queue = Object.values(collection.dict);

    const processor = async () => {
      while (queue.length > 0) {
        const album = queue.shift();
        const url = collection.getThumbnailUrl(album);
        if (!url || this.pinnedTextures.has(url) || this.inFlight.has(url) || this.failedUrls.has(url)) continue;
        
        this.inFlight.add(url);
        try {
          const res = await fetch(url);
          if (!res.ok) throw new Error("Failed to load");
          const blob = await res.blob();
          const bitmap = await createImageBitmap(blob, {
            premultiplyAlpha: 'none',
            colorSpaceConversion: 'default'
          });
          this.pinnedTextures.set(url, bitmap);
          this.scheduleFlush();
        } catch (err) {
          this.failedUrls.add(url);
        }
        this.inFlight.delete(url);
      }
    };

    const workers = Array.from({ length: concurrencyLimit }, () => processor());
    await Promise.all(workers);
    this.flush();
  }
}

export const prewarmer = new Prewarmer();
