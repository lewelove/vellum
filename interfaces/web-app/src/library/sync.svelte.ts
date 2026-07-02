import { connectSocket } from "../api.ts";

class SyncEngine extends EventTarget {
  _ws: WebSocket | null = null;
  isOpen: boolean = $state(false);

  connect() {
    this._ws = connectSocket(
      () => {
        this.isOpen = true;
        this.dispatchEvent(new Event('open'));
      },
      (event: MessageEvent) => {
        const process = (data: any) => {
          this.dispatchEvent(new CustomEvent('message', { detail: data }));
        };
        if (event.data instanceof Blob) {
          const reader = new FileReader();
          reader.onload = () => {
            try { process(JSON.parse(reader.result as string)); } catch (err) {}
          };
          reader.readAsText(event.data);
        } else {
          try { process(JSON.parse(event.data)); } catch (err) {}
        }
      }
    );
  }

  send(payload: any) {
    if (this.isOpen && this._ws) {
      this._ws.send(JSON.stringify(payload));
    }
  }
}

export const sync = new SyncEngine();
