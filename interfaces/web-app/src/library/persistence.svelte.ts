import { nav } from "../navigation.svelte.ts";
import type { ViewState } from "./view.svelte.ts";

export function applyPersistedState(state: any, view: ViewState) {
    nav.activeTab = state.activeTab || "home";
    view.homeSubView = state.homeSubView || "library";
    view.librariesState = state.librariesState || {};
    view.activeLibrary = state.activeLibrary || state.activeCollection || "library";
    
    view.loadLibraryState(view.activeLibrary);

    if (state.activeLibraryFilter !== undefined) view.activeLibraryFilter = state.activeLibraryFilter;
    if (state.groupKey !== undefined) view.activeSidebarGrouper = state.groupKey;
    if (state.filter !== undefined) view.activeFilter = state.filter;
    if (state.sortKey !== undefined) {
        view.userSortPreference = state.sortKey;
        view.activeSort = { key: state.sortKey, order: view.activeSort.order };
    }
    if (state.sortOrder !== undefined) {
        view.userSortOrder = state.sortOrder;
        view.activeSort = { key: view.activeSort.key, order: state.sortOrder };
    }

    view.activeShelf = state.activeShelf || null;
    view.isShaderEnabled = state.isShaderEnabled ?? true;
    view.sidebarWidth = state.sidebarWidth || 280;
}

export function persistState(view: ViewState) {
    view.saveCurrentLibraryState();
    fetch("/api/state", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({
            activeTab: nav.activeTab,
            homeSubView: view.homeSubView,
            activeLibrary: view.activeLibrary,
            librariesState: $state.snapshot(view.librariesState),
            activeLibraryFilter: view.activeLibraryFilter,
            sortKey: view.userSortPreference,
            sortOrder: view.userSortOrder,
            groupKey: view.activeSidebarGrouper,
            filter: $state.snapshot(view.activeFilter),
            activeShelf: view.activeShelf,
            isShaderEnabled: view.isShaderEnabled,
            sidebarWidth: view.sidebarWidth
        })
    }).catch(err => console.error(err));
}
