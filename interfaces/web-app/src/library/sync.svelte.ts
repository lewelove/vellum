import { connectSocket } from "../api.ts";

export class SyncEngine {
  _ws: WebSocket | null = null;
  _pendingViewReset: boolean = false;

  init(onOpen: () => void, onMessage: (msg: any) => void) {
    this._ws = connectSocket(
      onOpen,
      (event: MessageEvent) => {
        if (event.data instanceof Blob) {
          const reader = new FileReader();
          reader.onload = () => {
            try {
              onMessage(JSON.parse(reader.result as string));
            } catch (err) {
              console.error(err);
            }
          };
          reader.readAsText(event.data);
        } else {
          try {
            onMessage(JSON.parse(event.data));
          } catch (err) {
            console.error(err);
          }
        }
      }
    );
  }

  send(payload: any) {
    if (!this.isOpen) return;
    this._ws!.send(JSON.stringify(payload));
  }
  
  get isOpen() {
    return this._ws && this._ws.readyState === WebSocket.OPEN;
  }
}
