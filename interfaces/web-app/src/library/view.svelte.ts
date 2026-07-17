import { sync } from "./sync.svelte.ts";
import { collection } from "./collection.svelte.ts";
import { nav } from "../navigation.svelte.ts";
import { player } from "../modules/player.svelte.ts";
import { applyPersistedState, persistState } from "./persistence.svelte.ts";

export class ViewState {
  isLoading: boolean = $state(true);
  isConnected: boolean = $state(false);
  homeSubView: "library" | "shelves" = $state("library");
  
  focusedAlbum: any = $state(null);
  
  activeLibrary: string = $state("library");
  activeLibraryFilter: string | null = $state(null);
  activeFilter: { key: string | null, val: string | null } = $state({ key: null, val: null });
  activeSort: { key: string, order: string } = $state({ key: "default", order: "default" });
  userSortPreference: string = $state("default");
  userSortOrder: string = $state("default");
  activeSidebarGrouper: string = $state("genre");
  activeShelf: string | null = $state(null);
  librariesState: Record<string, any> = $state({});
  libraryVersion: number = $state(0);
  shelfVersion: number = $state(0);
  isShaderEnabled: boolean = $state(true);
  assetVersion: number = $state(Date.now());
  sidebarWidth: number = $state(280);
  isFocusInstant: boolean = $state(false);
  _pendingViewReset: boolean = false;

  constructor() {
    sync.addEventListener('open', () => { this.isConnected = true; });
    sync.addEventListener('message', (e: Event) => this.handleMessage((e as CustomEvent).detail));
  }

  handleMessage(json: any) {
    if (json.type === "INIT_DICT") {
      if (json.ui_state) {
        applyPersistedState(json.ui_state, this);
        const libDef = collection.availableLibraries[this.activeLibrary];
        if (libDef) {
            if (libDef.allowed_filters && !libDef.allowed_filters.includes(this.activeLibraryFilter)) {
                this.activeLibraryFilter = libDef.allowed_filters.length > 0 ? libDef.allowed_filters[0] : null;
            } else if (!libDef.allowed_filters || libDef.allowed_filters.length === 0) {
                this.activeLibraryFilter = null;
            }

            if (libDef.allowed_groupers && !libDef.allowed_groupers.includes(this.activeSidebarGrouper)) {
                this.activeSidebarGrouper = libDef.allowed_groupers[0] || (collection.manifest.groupers_order && collection.manifest.groupers_order[0]) || Object.keys(collection.availableFacets)[0] || "genre";
            }
            if (libDef.allowed_orders && !libDef.allowed_orders.includes(this.userSortPreference)) {
                this.userSortPreference = libDef.allowed_orders[0] || (collection.manifest.orders_order && collection.manifest.orders_order[0]) || Object.keys(collection.availableOrders)[0] || "default";
                this.activeSort = { key: this.userSortPreference, order: this.userSortOrder };
            }
        }
      }
      this.refreshView(true);
      this.refreshSidebar();
    } else if (json.type === "VIEW_DATA") {
      if (this._pendingViewReset) this.libraryVersion++;
      this.isLoading = false;
      this._pendingViewReset = false;
    } else if (json.type === "INTERFACE_ASSET_UPDATE" || json.type === "INTERFACE_CONFIG_UPDATE") {
      this.assetVersion = Date.now();
    } else if (json.type === "LOGIC_UPDATE") {
      window.location.reload();
    } else if (json.type === "ALBUM_REMOVED" || json.type === "ALBUM_UPDATED") {
      if (this.focusedAlbum && this.focusedAlbum.id === json.id) {
        if (json.type === "ALBUM_REMOVED") this.focusedAlbum = null;
        else collection.ensureFullAlbum(json.id).then(data => { if (data) this.focusedAlbum = data; });
      }
      this.refreshView(false);
      this.refreshSidebar();
    }
  }

  get isShaderActive() { return this.isShaderEnabled && player.state !== "stop"; }

  get shelfViewIds() {
    const shelfKey = this.activeShelf || (collection.manifest.shelves_order && collection.manifest.shelves_order[0]) || Object.keys(collection.availableShelves)[0];
    return shelfKey ? (collection.sidebarShelves[shelfKey] || []) : [];
  }

  get libraryAlbums() { return collection.mapIdsToAlbums(collection.libraryViewIds); }
  get shelfAlbums() { return collection.mapIdsToAlbums(this.shelfViewIds); }

  getSidebarGroup(key: string): any[] {
    if (!collection.sidebarGroups.has(key) && sync.isOpen) {
        this.refreshSidebar();
        return [];
    }
    return collection.sidebarGroups.get(key) || [];
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
    if (!this.librariesState[key]) {
      this.librariesState[key] = {};
    }
    const state = this.librariesState[key];
    const libraryDef = collection.availableLibraries[key] || {};
    const allowedFilters = libraryDef.allowed_filters || [];
    const allowedGroupers = libraryDef.allowed_groupers || [];
    const allowedOrders = libraryDef.allowed_orders || [];

    if (!state.activeLibraryFilter || !allowedFilters.includes(state.activeLibraryFilter)) {
      state.activeLibraryFilter = allowedFilters.length > 0 ? allowedFilters[0] : null;
    }
    if (!state.activeSidebarGrouper || !allowedGroupers.includes(state.activeSidebarGrouper)) {
      state.activeSidebarGrouper = allowedGroupers[0] || (collection.manifest.groupers_order && collection.manifest.groupers_order[0]) || Object.keys(collection.availableFacets)[0] || "genre";
    }
    if (!state.userSortPreference || !allowedOrders.includes(state.userSortPreference)) {
      state.userSortPreference = allowedOrders[0] || (collection.manifest.orders_order && collection.manifest.orders_order[0]) || Object.keys(collection.availableOrders)[0] || "default";
    }
    if (!state.userSortOrder) state.userSortOrder = "default";
    if (!state.activeSort) state.activeSort = { key: state.userSortPreference, order: state.userSortOrder };
    if (!state.activeFilter) state.activeFilter = { key: null, val: null };

    this.activeLibraryFilter = state.activeLibraryFilter;
    this.activeSidebarGrouper = state.activeSidebarGrouper;
    this.userSortPreference = state.userSortPreference;
    this.userSortOrder = state.userSortOrder;
    this.activeSort = { ...state.activeSort };
    this.activeFilter = { ...state.activeFilter };
  }

  refreshView(resetScroll: boolean = true) {
    if (!sync.isOpen) return;
    this._pendingViewReset = resetScroll;
    
    if (nav.activeTab === "home" && this.homeSubView === "shelves") {
        if (resetScroll) this.shelfVersion++;
        this.isLoading = false;
        this._pendingViewReset = false;
    } else {
        sync.send({
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
    if (!sync.isOpen) return;
    sync.send({
        type: "GROUP_REQUEST",
        library: this.activeLibrary,
        library_filter: this.activeLibraryFilter,
        key: this.activeSidebarGrouper
    });
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

  async setFocus(album: any, instant: boolean = false) {
    this.isFocusInstant = instant;
    this.focusedAlbum = await collection.ensureFullAlbum(album.id);
  }

  closeFocus() {
    this.focusedAlbum = null;
    this.isFocusInstant = false;
  }

  toggleShader() {
    this.isShaderEnabled = !this.isShaderEnabled;
    this.persistState();
  }

  persistState() {
    persistState(this);
  }
}

export const view = new ViewState();
