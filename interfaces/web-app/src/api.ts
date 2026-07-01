export function connectSocket(onOpen?: () => void, onMessage?: (e: MessageEvent) => void): WebSocket {
  const protocol = 'ws:';
  const host = '127.0.0.1:8000'; 
  const url = `${protocol}//${host}/ws`;

  let socket = new WebSocket(url);

  socket.onopen = () => {
    console.log("Vellum WebSocket: Connected to backend");
    if (onOpen) onOpen();
  };

  socket.onmessage = (event: MessageEvent) => {
    if (onMessage) onMessage(event);
  };

  socket.onclose = () => {
    console.log("Vellum WebSocket: Disconnected. Reconnecting...");
    setTimeout(() => {
      connectSocket(onOpen, onMessage);
    }, 2000);
  };

  socket.onerror = (err: Event) => {
    console.error("Vellum WebSocket: Error", err);
  };

  return socket;
}

export async function playAlbum(id: string, offset: number = 0): Promise<any> {
  const encodedId = encodeURIComponent(id);
  const response = await fetch(`/api/play/${encodedId}?offset=${offset}`, { method: "POST" });
  return await response.json();
}

export async function playDisc(id: string, discNumber: number, offset: number = 0): Promise<any> {
  const encodedId = encodeURIComponent(id);
  const response = await fetch(`/api/play-disc/${encodedId}?disc=${discNumber}&offset=${offset}`, { method: "POST" });
  return await response.json();
}

export async function queueAlbum(id: string): Promise<any> {
  const encodedId = encodeURIComponent(id);
  const response = await fetch(`/api/queue/${encodedId}`, { method: "POST" });
  return await response.json();
}

export async function jumpToQueueIndex(index: string | number): Promise<any> {
  const response = await fetch(`/api/jump/${index}`, { method: "POST" });
  return await response.json();
}

export async function openAlbumFolder(id: string): Promise<any> {
  const encodedId = encodeURIComponent(id);
  const response = await fetch(`/api/open/${encodedId}`, { method: "POST" });
  return await response.json();
}

export async function openLockFile(id: string): Promise<any> {
  const encodedId = encodeURIComponent(id);
  const response = await fetch(`/api/open-lock/${encodedId}`, { method: "POST" });
  return await response.json();
}

export async function openManifestFile(id: string): Promise<any> {
  const encodedId = encodeURIComponent(id);
  const response = await fetch(`/api/open-manifest/${encodedId}`, { method: "POST" });
  return await response.json();
}

export async function updateAlbum(id: string): Promise<any> {
  const encodedId = encodeURIComponent(id);
  const response = await fetch(`/api/update-album/${encodedId}`, { method: "POST" });
  return await response.json();
}
