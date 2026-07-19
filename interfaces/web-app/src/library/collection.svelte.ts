import { sync } from "./sync.svelte.ts";
import { updatePlayerState } from "../modules/player.svelte.ts";
import { config as globalConfig, updateConfig } from "../config.svelte.ts";

class CollectionStore {
  dict: Record<string, any> = $state({});
  trackPathMap: Record<string, any> = $state({});
  sidebarShelves: Record<string, string[]> = $state({});
  libraryViewIds: string[] = $state([]);
  sidebarGroups: Map<string, any[]> = $state(new Map());
  fullAlbumCache: Record<string, any> = $state({});
  manifest: Record<string, any> = $state({ filters: {}, libraries: {}, groupers: {}, orders: {}, shelves: {} });
  config: Record<string, any> = $state({});

  constructor() {
    sync.addEventListener('message', (e: Event) => this.handleMessage((e as CustomEvent).detail));
  }

  handleMessage(json: any) {
    if (json.type === "INIT_DICT") {
      if (json.shelves) this.sidebarShelves = json.shelves;
      this.dict = json.dict || {};
      this.trackPathMap = json.trackMap || {};
      if (json.manifest) this.manifest = json.manifest;
      if (json.config) this.config = { ...this.config, ...json.config };
    } else if (json.type === "VIEW_DATA") {
      this.libraryViewIds = json.ids || [];
    } else if (json.type === "GROUP_RESULT") {
      const newMap = new Map(this.sidebarGroups);
      newMap.set(json.key, json.result);
      this.sidebarGroups = newMap;
    } else if (json.type === "ALBUM_REMOVED") {
      if (json.shelves) this.sidebarShelves = json.shelves;
      delete this.dict[json.id];
      delete this.fullAlbumCache[json.id];
    } else if (json.type === "ALBUM_UPDATED") {
      if (json.shelves) this.sidebarShelves = json.shelves;
      if (json.dictEntry && Object.keys(json.dictEntry).length > 0) {
        this.dict[json.id] = json.dictEntry;
      } else {
        delete this.dict[json.id];
      }
      delete this.fullAlbumCache[json.id];
    } else if (json.type === "CONFIG_UPDATE" || json.type === "INTERFACE_CONFIG_UPDATE") {
      if (json.config) {
        this.config = { ...this.config, ...json.config };
        if (json.type === "INTERFACE_CONFIG_UPDATE") updateConfig(json.config);
      }
    } else if (json.type === "MPD_STATUS") {
      updatePlayerState(json);
    }
  }

  mapIdsToAlbums(ids: string[]): any[] {
    return ids.map(id => {
      let a = this.dict[id];
      return a ? {
          id: a.id,
          title: a.album,
          artist: a.albumartist,
          cover_hash: a.cover_hash,
          total_discs: a.total_discs,
          total_tracks: a.total_tracks,
          duration_formatted: a.duration_formatted,
          virtual: a.virtual,
          keys: a.keys
      } : null;
    }).filter(Boolean);
  }

  getThumbnailUrl(album: any): string {
    if (!album || !album.cover_hash) return "";
    const algo = globalConfig.album_grid.album_card.cover.filter || "lanczos";
    const size = globalConfig.album_grid.album_card.cover.size || 200;
    return `/api/covers/${algo}/${size}px/${album.cover_hash}`;
  }

  getAlbumCoverUrl(albumId: string): string {
    const album = this.dict[albumId];
    if (!album || !album.cover_hash) return "";
    return `/api/assets/cover/${encodeURIComponent(albumId)}?v=${album.cover_hash}`;
  }

  getTrackByPath(path: string): any {
    return this.trackPathMap[path];
  }

  async ensureFullAlbum(id: string): Promise<any> {
    if (!id) return null;
    if (this.fullAlbumCache[id]) return this.fullAlbumCache[id];
    try {
        const res = await fetch(`/api/album/${encodeURIComponent(id)}`);
        if (res.ok) {
            const data = await res.json();
            data.id = id;
            this.fullAlbumCache[id] = data;
            return data;
        }
    } catch (err) {
        console.error(err);
    }
    return null;
  }

  get availableFilters(): Record<string, any> { return this.manifest.filters || {}; }
  get availableLibraries(): Record<string, any> { return this.manifest.libraries || {}; }
  get availableFacets(): Record<string, any> { return this.manifest.groupers || {}; }
  get availableOrders(): Record<string, any> { return this.manifest.orders || {}; }
  get availableShelves(): Record<string, any> { return this.manifest.shelves || {}; }

  get librariesList(): any[] {
    const order = this.manifest.libraries_order || Object.keys(this.availableLibraries);
    return order.map((k: string) => ({ key: k, ...this.availableLibraries[k] }));
  }

  get shelvesList(): any[] {
    const order = this.manifest.shelves_order || Object.keys(this.availableShelves);
    return order.map((k: string) => ({ key: k, ...this.availableShelves[k] }));
  }

  getVisibleFacets(activeLibrary: string): any[] {
    const library = this.availableLibraries[activeLibrary];
    const order = this.manifest.groupers_order || Object.keys(this.availableFacets);
    if (library && library.allowed_groupers) {
      return library.allowed_groupers
        .filter((k: string) => this.availableFacets[k])
        .map((k: string) => ({ key: k, label: this.availableFacets[k].label || k }));
    }
    return order
      .filter((k: string) => this.availableFacets[k])
      .map((k: string) => ({ key: k, label: this.availableFacets[k].label || k }));
  }

  getVisibleOrders(activeLibrary: string): any[] {
    const library = this.availableLibraries[activeLibrary];
    const order = this.manifest.orders_order || Object.keys(this.availableOrders);
    if (library && library.allowed_orders) {
      return library.allowed_orders
        .filter((k: string) => this.availableOrders[k])
        .map((k: string) => ({ key: k, label: this.availableOrders[k].label || k }));
    }
    return order
      .filter((k: string) => this.availableOrders[k])
      .map((k: string) => ({ key: k, label: this.availableOrders[k].label || k }));
  }
}

export const collection = new CollectionStore();
