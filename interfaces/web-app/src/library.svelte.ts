import { updatePlayerState } from "./modules/player.svelte.ts";
import { nav } from "./navigation.svelte.ts";
import { updateTheme } from "./theme.svelte.ts";

import { CollectionStore } from "./library/collection.svelte.ts";
import { ViewState } from "./library/view.svelte.ts";
import { SyncEngine } from "./library/sync.svelte.ts";
import { Prewarmer } from "./library/prewarmer.svelte.ts";
import { PersistenceHandler } from "./library/persistence.svelte.ts";

class LibraryState {
  _collection = new CollectionStore();
  _view = new ViewState();
  _sync = new SyncEngine();
  _prewarmer = new Prewarmer();
  _persistence = new PersistenceHandler();

  get dict() { return this._collection.dict; }
  set dict(v) { this._collection.dict = v; }

  get trackPathMap() { return this._collection.trackPathMap; }
  set trackPathMap(v) { this._collection.trackPathMap = v; }

  get sidebarShelves() { return this._collection.sidebarShelves; }
  set sidebarShelves(v) { this._collection.sidebarShelves = v; }

  get libraryViewIds() { return this._collection.libraryViewIds; }
  set libraryViewIds(v) { this._collection.libraryViewIds = v; }

  get sidebarGroups() { return this._collection.sidebarGroups; }
  set sidebarGroups(v) { this._collection.sidebarGroups = v; }

  get fullAlbumCache() { return this._collection.fullAlbumCache; }
  set fullAlbumCache(v) { this._collection.fullAlbumCache = v; }

  get manifest() { return this._collection.manifest; }
  set manifest(v) { this._collection.manifest = v; }

  get config() { return this._collection.config; }
  set config(v) { this._collection.config = v; }

  get shelfViewIds() {
      const shelfKey = this.activeShelf || (this.manifest.shelves_order && this.manifest.shelves_order[0]) || Object.keys(this.availableShelves)[0];
      return shelfKey ? (this.sidebarShelves[shelfKey] || []) : [];
  }

  get libraryAlbums() { return this._collection.mapIdsToAlbums(this.libraryViewIds); }
  get shelfAlbums() { return this._collection.mapIdsToAlbums(this.shelfViewIds); }

  get isLoading() { return this._view.isLoading; }
  set isLoading(v) { this._view.isLoading = v; }

  get isConnected() { return this._view.isConnected; }
  set isConnected(v) { this._view.isConnected = v; }

  get homeSubView() { return this._view.homeSubView; }
  set homeSubView(v) { this._view.homeSubView = v; }

  get focusedAlbums() { return this._view.focusedAlbums; }
  set focusedAlbums(v) { this._view.focusedAlbums = v; }

  get focusedAlbum() { return this._view.focusedAlbum; }
  set focusedAlbum(v) { this._view.focusedAlbum = v; }

  get activeLibrary() { return this._view.activeLibrary; }
  set activeLibrary(v) { this._view.activeLibrary = v; }

  get activeLibraryFilter() { return this._view.activeLibraryFilter; }
  set activeLibraryFilter(v) { this._view.activeLibraryFilter = v; }

  get activeFilter() { return this._view.activeFilter; }
  set activeFilter(v) { this._view.activeFilter = v; }

  get activeSort() { return this._view.activeSort; }
  set activeSort(v) { this._view.activeSort = v; }

  get userSortPreference() { return this._view.userSortPreference; }
  set userSortPreference(v) { this._view.userSortPreference = v; }

  get userSortOrder() { return this._view.userSortOrder; }
  set userSortOrder(v) { this._view.userSortOrder = v; }

  get activeSidebarGrouper() { return this._view.activeSidebarGrouper; }
  set activeSidebarGrouper(v) { this._view.activeSidebarGrouper = v; }

  get activeShelf() { return this._view.activeShelf; }
  set activeShelf(v) { this._view.activeShelf = v; }

  get librariesState() { return this._view.librariesState; }
  set librariesState(v) { this._view.librariesState = v; }

  get libraryVersion() { return this._view.libraryVersion; }
  set libraryVersion(v) { this._view.libraryVersion = v; }

  get shelfVersion() { return this._view.shelfVersion; }
  set shelfVersion(v) { this._view.shelfVersion = v; }

  get themeVersion() { return this._view.themeVersion; }
  set themeVersion(v) { this._view.themeVersion = v; }

  get sidebarWidth() { return this._view.sidebarWidth; }
  set sidebarWidth(v) { this._view.sidebarWidth = v; }

  get isShaderEnabled() { return this._view.isShaderEnabled; }
  set isShaderEnabled(v) { this._view.isShaderEnabled = v; }

  get isShaderActive() { return this._view.isShaderActive; }

  get queuePanels() { return this._view.queuePanels; }
  set queuePanels(v) { this._view.queuePanels = v; }

  get pinnedTextures() { return this._prewarmer.pinnedTextures; }
  set pinnedTextures(v) { this._prewarmer.pinnedTextures = v; }

  get _pendingViewReset() { return this._sync._pendingViewReset; }
  set _pendingViewReset(v) { this._sync._pendingViewReset = v; }

  init() {
    this._sync.init(
      () => { this.isConnected = true; },
      (json) => this.dispatchSocketAction(json)
    );

    fetch("/api/interfaces/default/config")
        .then(res => res.json())
        .then(data => {
            this.config = { ...this.config, ...data };
            updateTheme(data);
        })
        .catch(() => {
            fetch("/api/interfaces/web-app/config")
                .then(res => res.json())
                .then(data => {
                    this.config = { ...this.config, ...data };
                    updateTheme(data);
                })
                .catch(e => console.error(e));
        });
  }

  dispatchSocketAction(json: any) {
    if (json.type === "INIT_DICT") {
      if (json.shelves) this.sidebarShelves = json.shelves;
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
      this.libraryViewIds = json.ids || [];
      if (this._pendingViewReset) this.libraryVersion++;
      
      this.isLoading = false;
      this._pendingViewReset = false;
    } else if (json.type === "GROUP_RESULT") {
      const newMap = new Map(this.sidebarGroups);
      newMap.set(json.key, json.result);
      this.sidebarGroups = newMap;
    } else if (json.type === "MPD_STATUS") {
      updatePlayerState(json);
    } else if (json.type === "INTERFACE_ASSET_UPDATE") {
        if (json.name === "web-app" || json.name === "default") {
            this.themeVersion = Date.now();
        }
    } else if (json.type === "INTERFACE_CONFIG_UPDATE") {
        if (json.name === "web-app" || json.name === "default") {
            this.config = { ...this.config, ...json.config };
            updateTheme(json.config);
            this.themeVersion = Date.now();
            this.orchestratePrewarming();
        }
    } else if (json.type === "LOGIC_UPDATE") {
      window.location.reload(); 
    } else if (json.type === "ALBUM_REMOVED") {
      if (json.shelves) this.sidebarShelves = json.shelves;
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
      if (json.shelves) this.sidebarShelves = json.shelves;
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

  orchestratePrewarming() {
    this._prewarmer.orchestrate(this.dict, (album) => this.getThumbnailUrl(album));
  }

  applyPersistedState(state: any) {
    this._persistence.applyPersistedState(state, this);
  }

  persistState() {
    this._persistence.persistState(this);
  }

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
    this.activeSort = { ...state.activeSort };
    this.activeFilter = { ...state.activeFilter };
  }

  refreshView(resetScroll: boolean = true) {
    if (!this._sync.isOpen) return;
    this._pendingViewReset = resetScroll;
    
    if (nav.activeTab === "home" && this.homeSubView === "shelves") {
        if (resetScroll) this.shelfVersion++;
        this.isLoading = false;
        this._pendingViewReset = false;
    } else {
        this._sync.send({
            type: "VIEW_REQUEST",
            library: this.activeLibrary,
            library_filter: this.activeLibraryFilter,
            sort: this.activeSort.key,
            reverse: this.activeSort.order === "reverse",
            filter: this.activeFilter
        });
    }
  }

  refreshSidebar() {
    if (!this._sync.isOpen) return;
    this._sync.send({
        type: "GROUP_REQUEST",
        library: this.activeLibrary,
        library_filter: this.activeLibraryFilter,
        key: this.activeSidebarGrouper
    });
  }

  getSidebarGroup(key: string): any[] {
    if (!this.sidebarGroups.has(key) && this._sync.isOpen) {
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
    this.shelfVersion++;
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
    const key = nav.activeTab === 'home' ? this.homeSubView : nav.activeTab;
    this.focusedAlbums[key] = await this.ensureFullAlbum(album.id);
  }

  closeFocus() {
    const key = nav.activeTab === 'home' ? this.homeSubView : nav.activeTab;
    this.focusedAlbums[key] = null;
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
