export const player = $state<{
  state: string;
  currentAlbumId: string | null;
  currentFile: string | null;
  title: string;
  artist: string;
  elapsed: number;
  duration: number;
  lastUpdated: number;
  queue: any[];
}>({
  state: "stop",
  currentAlbumId: null,
  currentFile: null,
  title: "",
  artist: "",
  elapsed: 0,
  duration: 0,
  lastUpdated: 0,
  queue:[]
});

export function updatePlayerState(data: any) {
  player.state = data.state;
  player.currentAlbumId = data.album_id;
  player.currentFile = data.file;
  player.title = data.title || "";
  player.artist = data.artist || "";
  player.elapsed = parseFloat(data.elapsed || 0);
  player.duration = parseFloat(data.duration || 0);
  player.lastUpdated = performance.now();
  
  if (data.queue) {
    player.queue = data.queue;
  }
}
