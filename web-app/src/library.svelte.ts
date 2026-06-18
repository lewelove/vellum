import { connectSocket } from "./api.ts";
import { player, updatePlayerState } from "./modules/player.svelte.ts";
import { nav } from "./navigation.svelte.ts";

class LibraryState {
  dict: Record<string, any> = $state({});
  trackPathMap: Record<string, any> = $state({});
  
  libraryViewIds: string[] = $state([]);
  shelfViewIds: string[] = $state([]);
  
  libraryAlbums = $derived(this.mapIdsToAlbums(this.libraryViewIds));
  shelfAlbums = $derived(this.mapIdsToAlbums(this.shelfViewIds));
  
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
          album_duration_time: a.album_duration_time,
          tags: a.tags
      } : null;
    }).filter(Boolean);
  }
  
  sidebarGroups: Map<string, any[]> = $state(new Map()); 
  isLoading: boolean = $state(true);
  isConnected: boolean = $state(false);
  
  focusedAlbums: Record<string, any> = $state({ home: null, shelves: null, queue: null });

  get focusedAlbum(): any {
    return this.focusedAlbums[nav.activeTab] || null;
  }

  set focusedAlbum(val: any) {
    this.focusedAlbums[nav.activeTab] = val;
  }
  
  activeLibrary: string = $state("library");
  activeLibraryFilter: string | null = $state(null);
  activeFilter: { key: string | null, val: string | null } = $state({ key: null, val: null });
  activeSort: { key: string, order: string } = $state({ key: "default", order: "default" });
  userSortPreference: string = $state("default");
  userSortOrder: string = $state("default");
  activeSidebarGrouper: string = $state("genre");
  activeShelf: string | null = $state(null);

  librariesState: Record<string, any> = $state({});

  getLibraryState(libKey: string) {
    if (!this.librariesState[libKey]) {
      this.librariesState[libKey] = {};
    }
    const state = this.librariesState[libKey];
    const libraryDef = this.availableLibraries[libKey] || {};
    const allowedFilters = libraryDef.allowed_filters || [];
    const allowedGroupers = libraryDef.allowed_groupers || [];
    const allowedOrders = libraryDef.allowed_orders || [];

    if (!state.activeLibraryFilter || !allowedFilters.includes(state.activeLibraryFilter)) {
      state.activeLibraryFilter = allowedFilters.length > 0 ? allowedFilters[0] : null;
    }

    if (!state.activeSidebarGrouper || !allowedGroupers.includes(state.activeSidebarGrouper)) {
      state.activeSidebarGrouper = allowedGroupers[0] || (this.manifest.groupers_order && this.manifest.groupers_order[0]) || Object.keys(this.availableFacets)[0] || "genre";
    }

    if (!state.userSortPreference || !allowedOrders.includes(state.userSortPreference)) {
      state.userSortPreference = allowedOrders[0] || (this.manifest.orders_order && this.manifest.orders_order[0]) || Object.keys(this.availableOrders)[0] || "default";
    }

    if (!state.userSortOrder) {
      state.userSortOrder = "default";
    }

    if (!state.activeSort) {
      state.activeSort = { key: state.userSortPreference, order: state.userSortOrder };
    }

    if (!state.activeFilter) {
      state.activeFilter = { key: null, val: null };
    }

    return state;
  }

  saveCurrentLibraryState() {
    if (!this.activeLibrary) return;
    this.librariesState[this.activeLibrary] = {
      activeLibraryFilter: this.activeLibraryFilter,
      activeFilter: $state.snapshot(this.activeFilter),
      userSortPreference: this.userSortPreference,
      userSortOrder: this.userSortOrder,
      activeSidebarGrouper: this.activeSidebarGrouper,
      activeSort: $state.snapshot(this.activeSort)
    };
  }

  loadLibraryState(key: string) {
    const state = this.getLibraryState(key);
    this.activeLibraryFilter = state.activeLibraryFilter;
    this.activeSidebarGrouper = state.activeSidebarGrouper;
    this.userSortPreference = state.userSortPreference;
    this.userSortOrder = state.userSortOrder;
    this.activeSort = $state.snapshot(state.activeSort);
    this.activeFilter = $state.snapshot(state.activeFilter);
  }
  
  libraryVersion: number = $state(0);
  shelfVersion: number = $state(0);

  pinnedTextures: Map<string, ImageBitmap> = $state(new Map());
  fullAlbumCache: Record<string, any> = $state({});
  isShaderEnabled: boolean = $state(true);
  isShaderActive: boolean = $derived(this.isShaderEnabled && player.state !== "stop");
  queuePanels: Record<string, boolean> = $state({ hud: true });
  themeVersion: number = $state(Date.now());
  
  sidebarWidth: number = $state(280);
  
  manifest: Record<string, any> = $state({ filters: {}, libraries: {}, groupers: {}, orders: {}, shelves: {} });

  config: Record<string, any> = $state({
    covers: {
        master: { interpolation: "mitchell", size: 1080 },
        thumbnail: { interpolation: "lanczos", size: 190 }
    },
    shader: null
  });

  _ws: WebSocket | null = null;
  _pendingViewReset: boolean = false;

  init() {
    this._ws = connectSocket(
      () => { this.isConnected = true; },
      (event: MessageEvent) => this.handleSocketMessage(event)
    );
  }

  handleSocketMessage(event: MessageEvent) {
    if (event.data instanceof Blob) {
      const reader = new FileReader();
      reader.onload = () => {
        try {
          const json = JSON.parse(reader.result as string);
          this.dispatchSocketAction(json);
        } catch (err) {
          console.error(err);
        }
      };
      reader.readAsText(event.data);
    } else {
      try {
        const json = JSON.parse(event.data);
        this.dispatchSocketAction(json);
      } catch (err) {
        console.error(err);
      }
    }
  }

  dispatchSocketAction(json: any) {
    if (json.type === "INIT_DICT") {
      this.dict = json.dict || {};
      this.trackPathMap = json.trackMap || {};
      if (json.manifest) this.manifest = json.manifest;
      if (json.config) this.config = { ...this.config, ...json.config };
      if (json.ui_state) {
          this.applyPersistedState(json.ui_state);
          
          const libDef = this.availableLibraries[this.activeLibrary];
          if (libDef) {
              if (libDef.allowed_filters && !libDef.allowed_filters.includes(this.activeLibraryFilter)) {
                  this.activeLibraryFilter = libDef.allowed_filters.length > 0 ? libDef.allowed_filters[0] : null;
              } else if (!libDef.allowed_filters || libDef.allowed_filters.length === 0) {
                  this.activeLibraryFilter = null;
              }

              if (libDef.allowed_groupers && !libDef.allowed_groupers.includes(this.activeSidebarGrouper)) {
                  this.activeSidebarGrouper = libDef.allowed_groupers[0] || (this.manifest.groupers_order && this.manifest.groupers_order[0]) || Object.keys(this.availableFacets)[0] || "genre";
              }
              if (libDef.allowed_orders && !libDef.allowed_orders.includes(this.userSortPreference)) {
                  this.userSortPreference = libDef.allowed_orders[0] || (this.manifest.orders_order && this.manifest.orders_order[0]) || Object.keys(this.availableOrders)[0] || "default";
                  this.activeSort = { key: this.userSortPreference, order: this.userSortOrder };
              }
          }
      }
      
      this.orchestratePrewarming();
      this.refreshView(true);
      this.refreshSidebar();
      
    } else if (json.type === "VIEW_DATA") {
      const isShelves = (nav.activeTab === "shelves");
      
      if (isShelves) {
        this.shelfViewIds = json.ids || [];
        if (this._pendingViewReset) this.shelfVersion++;
      } else {
        this.libraryViewIds = json.ids || [];
        if (this._pendingViewReset) this.libraryVersion++;
      }
      
      this.isLoading = false;
      this._pendingViewReset = false;
    } else if (json.type === "GROUP_RESULT") {
      const newMap = new Map(this.sidebarGroups);
      newMap.set(json.key, json.result);
      this.sidebarGroups = newMap;
    } else if (json.type === "MPD_STATUS") {
      updatePlayerState(json);
    } else if (json.type === "THEME_UPDATE") {
      this.themeVersion = Date.now();
    } else if (json.type === "LOGIC_UPDATE") {
      window.location.reload(); 
    } else if (json.type === "ALBUM_REMOVED") {
      delete this.dict[json.id];
      delete this.fullAlbumCache[json.id];
      
      for (const tab of Object.keys(this.focusedAlbums)) {
        if (this.focusedAlbums[tab] && this.focusedAlbums[tab].id === json.id) {
          this.focusedAlbums[tab] = null;
        }
      }

      this.refreshView(false);
      this.refreshSidebar();
    } else if (json.type === "ALBUM_UPDATED") {
      if (json.dictEntry && Object.keys(json.dictEntry).length > 0) {
        this.dict[json.id] = json.dictEntry;
      } else {
        delete this.dict[json.id];
      }
      delete this.fullAlbumCache[json.id];
      
      for (const tab of Object.keys(this.focusedAlbums)) {
        if (this.focusedAlbums[tab] && this.focusedAlbums[tab].id === json.id) {
          this.ensureFullAlbum(json.id).then(data => {
            if (data) this.focusedAlbums[tab] = data;
          });
        }
      }

      this.orchestratePrewarming();
      this.refreshView(false);
      this.refreshSidebar();
    } else if (json.type === "CONFIG_UPDATE") {
      if (json.config) {
        this.config = { ...this.config, ...json.config };
        this.orchestratePrewarming();
      }
    }
  }

  async orchestratePrewarming() {
    const concurrencyLimit = 6;
    const queue = Object.values(this.dict);
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
        const url = this.getThumbnailUrl(album);
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

  applyPersistedState(state: any) {
      nav.activeTab = state.activeTab || "home";
      this.librariesState = state.librariesState || {};
      this.activeLibrary = state.activeLibrary || state.activeCollection || "library";
      
      this.loadLibraryState(this.activeLibrary);

      if (state.activeLibraryFilter !== undefined) this.activeLibraryFilter = state.activeLibraryFilter;
      if (state.groupKey !== undefined) this.activeSidebarGrouper = state.groupKey;
      if (state.filter !== undefined) this.activeFilter = state.filter;
      if (state.sortKey !== undefined) {
          this.userSortPreference = state.sortKey;
          this.activeSort.key = state.sortKey;
      }
      if (state.sortOrder !== undefined) {
          this.userSortOrder = state.sortOrder;
          this.activeSort.order = state.sortOrder;
      }

      this.activeShelf = state.activeShelf || null;
      this.isShaderEnabled = state.isShaderEnabled ?? true;
      this.sidebarWidth = state.sidebarWidth || 280;
      
      this.queuePanels = state.queuePanels || { hud: true };
      if (this.queuePanels.hud === undefined) {
        this.queuePanels.hud = this.queuePanels.control !== false;
        delete this.queuePanels.control;
        delete this.queuePanels.tracks;
        delete this.queuePanels.lyrics;
      }
  }

  persistState() {
      this.saveCurrentLibraryState();
      fetch("/api/state", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
              activeTab: nav.activeTab,
              activeLibrary: this.activeLibrary,
              librariesState: $state.snapshot(this.librariesState),
              activeLibraryFilter: this.activeLibraryFilter,
              sortKey: this.userSortPreference,
              sortOrder: this.userSortOrder,
              groupKey: this.activeSidebarGrouper,
              filter: $state.snapshot(this.activeFilter),
              activeShelf: this.activeShelf,
              isShaderEnabled: this.isShaderEnabled,
              queuePanels: $state.snapshot(this.queuePanels),
              sidebarWidth: this.sidebarWidth
          })
      }).catch(err => console.error(err));
  }

  refreshView(resetScroll: boolean = true) {
    if (!this._ws || this._ws.readyState !== WebSocket.OPEN) return;
    this._pendingViewReset = resetScroll;
    
    if (nav.activeTab === "shelves") {
        const firstShelf = (this.manifest.shelves_order && this.manifest.shelves_order[0]) || Object.keys(this.availableShelves)[0];
        this._ws.send(JSON.stringify({
            type: "SHELF_REQUEST",
            shelf: this.activeShelf || firstShelf
        }));
    } else {
        this._ws.send(JSON.stringify({
            type: "VIEW_REQUEST",
            library: this.activeLibrary,
            library_filter: this.activeLibraryFilter,
            sort: this.activeSort.key,
            reverse: this.activeSort.order === "reverse",
            filter: this.activeFilter
        }));
    }
  }

  refreshSidebar() {
    if (!this._ws || this._ws.readyState !== WebSocket.OPEN) return;
    this._ws.send(JSON.stringify({
        type: "GROUP_REQUEST",
        library: this.activeLibrary,
        library_filter: this.activeLibraryFilter,
        key: this.activeSidebarGrouper
    }));
  }

  getSidebarGroup(key: string): any[] {
    if (!this.sidebarGroups.has(key) && this._ws?.readyState === WebSocket.OPEN) {
        this.refreshSidebar();
        return [];
    }
    return this.sidebarGroups.get(key) || [];
  }

  getTrackByPath(path: string): any {
    return this.trackPathMap[path];
  }

  getThumbnailUrl(album: any): string {
    if (!album || !album.cover_hash) return "";
    const thumbConf = this.config.covers?.thumbnail || { interpolation: "lanczos", size: 190 };
    const algo = thumbConf.interpolation || "lanczos";
    const size = thumbConf.size || 190;
    return `/api/covers/${algo}/${size}px/${album.cover_hash}`;
  }

  getAlbumCoverUrl(albumId: string): string {
    const album = this.dict[albumId];
    if (!album || !album.cover_hash) return "";
    return `/api/assets/cover/${encodeURIComponent(albumId)}?v=${album.cover_hash}`;
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

  get visibleFacets(): any[] {
    const library = this.availableLibraries[this.activeLibrary];
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

  get visibleOrders(): any[] {
    const library = this.availableLibraries[this.activeLibrary];
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

  setLibrary(key: string) {
    this.saveCurrentLibraryState();
    this.activeLibrary = key;
    this.loadLibraryState(key);
    this.refreshView(true);
    this.refreshSidebar();
    this.persistState();
  }

  setLibraryFilter(key: string) {
    this.activeLibraryFilter = key;
    this.activeFilter = { key: null, val: null };
    this.refreshView(true);
    this.refreshSidebar();
    this.persistState();
  }

  setShelf(key: string) {
    this.activeShelf = key;
    this.focusedAlbum = null;
    this.refreshView(true);
    this.persistState();
  }

  setSidebarGrouper(key: string) {
    this.activeSidebarGrouper = key;
    this.refreshSidebar();
    this.persistState();
  }

  applyFilter(key: string, val: string) {
    if (this.activeFilter.key === key && this.activeFilter.val === val) {
      this.activeFilter = { key: null, val: null };
    } else {
      this.activeFilter = { key, val };
    }
    this.focusedAlbum = null;
    this.activeSort = { key: this.userSortPreference, order: this.userSortOrder };
    this.refreshView(true);
    this.persistState();
  }

  applySort(key: string) {
    this.activeSort = { key, order: "default" };
    this.refreshView(true);
  }

  setUserSort(key: string) {
    this.userSortPreference = key;
    this.activeSort = { key, order: this.userSortOrder };
    this.refreshView(true);
    this.persistState();
  }

  toggleSortOrder() {
    this.userSortOrder = (this.userSortOrder === "default") ? "reverse" : "default";
    this.activeSort = { key: this.userSortPreference, order: this.userSortOrder };
    this.refreshView(true);
    this.persistState();
  }

  restoreUserSort() {
    this.activeSort = { key: this.userSortPreference, order: this.userSortOrder };
    this.refreshView(true);
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

  async setFocus(album: any) {
    this.focusedAlbums[nav.activeTab] = await this.ensureFullAlbum(album.id);
  }

  closeFocus() {
    this.focusedAlbums[nav.activeTab] = null;
  }
  
  toggleShader() {
    this.isShaderEnabled = !this.isShaderEnabled;
    this.persistState();
  }

  toggleQueuePanel(key: string) {
    this.queuePanels[key] = !this.queuePanels[key];
    this.persistState();
  }
}

export const library = new LibraryState();
