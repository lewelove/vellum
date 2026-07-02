import { nav } from "../navigation.svelte.ts";
import { player } from "../modules/player.svelte.ts";

export class ViewState {
  isLoading: boolean = $state(true);
  isConnected: boolean = $state(false);

  homeSubView: "library" | "shelves" = $state("library");
  focusedAlbums: Record<string, any> = $state({ library: null, shelves: null, queue: null });

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
  queuePanels: Record<string, boolean> = $state({ hud: true });
  themeVersion: number = $state(Date.now());
  sidebarWidth: number = $state(280);

  get isShaderActive(): boolean {
    return this.isShaderEnabled && player.state !== "stop";
  }

  get focusedAlbum(): any {
    const key = nav.activeTab === 'home' ? this.homeSubView : nav.activeTab;
    return this.focusedAlbums[key] || null;
  }

  set focusedAlbum(val: any) {
    const key = nav.activeTab === 'home' ? this.homeSubView : nav.activeTab;
    this.focusedAlbums[key] = val;
  }
}
