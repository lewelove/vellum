import { sync } from "./sync.svelte.ts";
import { collection } from "./collection.svelte.ts";
import { prewarmer } from "./prewarmer.svelte.ts";
import { updateTheme } from "../theme.svelte.ts";

export function initApp() {
  prewarmer; // Touch prewarmer to ensure it evaluates and attaches listeners

  sync.connect();
  
  fetch("/api/interfaces/default/config")
    .then(res => res.json())
    .then(data => {
        collection.config = { ...collection.config, ...data };
        updateTheme(data);
    })
    .catch(() => {
        fetch("/api/interfaces/web-app/config")
            .then(res => res.json())
            .then(data => {
                collection.config = { ...collection.config, ...data };
                updateTheme(data);
            })
            .catch(e => console.error(e));
    });
}
