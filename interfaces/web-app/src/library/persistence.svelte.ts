import { nav } from "../navigation.svelte.ts";

export class PersistenceHandler {
  applyPersistedState(state: any, facade: any) {
      nav.activeTab = state.activeTab || "home";
      facade.homeSubView = state.homeSubView || "library";
      facade.librariesState = state.librariesState || {};
      facade.activeLibrary = state.activeLibrary || state.activeCollection || "library";
      
      facade.loadLibraryState(facade.activeLibrary);

      if (state.activeLibraryFilter !== undefined) facade.activeLibraryFilter = state.activeLibraryFilter;
      if (state.groupKey !== undefined) facade.activeSidebarGrouper = state.groupKey;
      if (state.filter !== undefined) facade.activeFilter = state.filter;
      if (state.sortKey !== undefined) {
          facade.userSortPreference = state.sortKey;
          facade.activeSort = { key: state.sortKey, order: facade.activeSort.order };
      }
      if (state.sortOrder !== undefined) {
          facade.userSortOrder = state.sortOrder;
          facade.activeSort = { key: facade.activeSort.key, order: state.sortOrder };
      }

      facade.activeShelf = state.activeShelf || null;
      facade.isShaderEnabled = state.isShaderEnabled ?? true;
      facade.sidebarWidth = state.sidebarWidth || 280;
      
      facade.queuePanels = state.queuePanels || { hud: true };
      if (facade.queuePanels.hud === undefined) {
        facade.queuePanels.hud = facade.queuePanels.control !== false;
        delete facade.queuePanels.control;
        delete facade.queuePanels.tracks;
        delete facade.queuePanels.lyrics;
      }
  }

  persistState(facade: any) {
      facade.saveCurrentLibraryState();
      fetch("/api/state", {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify({
              activeTab: nav.activeTab,
              homeSubView: facade.homeSubView,
              activeLibrary: facade.activeLibrary,
              librariesState: $state.snapshot(facade.librariesState),
              activeLibraryFilter: facade.activeLibraryFilter,
              sortKey: facade.userSortPreference,
              sortOrder: facade.userSortOrder,
              groupKey: facade.activeSidebarGrouper,
              filter: $state.snapshot(facade.activeFilter),
              activeShelf: facade.activeShelf,
              isShaderEnabled: facade.isShaderEnabled,
              queuePanels: $state.snapshot(facade.queuePanels),
              sidebarWidth: facade.sidebarWidth
          })
      }).catch(err => console.error(err));
  }
}
