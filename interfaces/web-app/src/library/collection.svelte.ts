export class CollectionStore {
  dict: Record<string, any> = $state({});
  trackPathMap: Record<string, any> = $state({});
  sidebarShelves: Record<string, string[]> = $state({});
  libraryViewIds: string[] = $state([]);
  sidebarGroups: Map<string, any[]> = $state(new Map());
  fullAlbumCache: Record<string, any> = $state({});
  manifest: Record<string, any> = $state({ filters: {}, libraries: {}, groupers: {}, orders: {}, shelves: {} });
  config: Record<string, any> = $state({
    covers: {
        master: { interpolation: "mitchell", size: 1080 },
        thumbnail: { interpolation: "lanczos", size: 190 }
    }
  });

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
          keys: a.keys
      } : null;
    }).filter(Boolean);
  }
}
