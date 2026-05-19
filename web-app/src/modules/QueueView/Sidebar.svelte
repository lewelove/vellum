<script lang="ts">
  import { library } from "../../library.svelte.ts";
  import { player } from "../player.svelte.ts";
  import { 
    playAlbum, 
    openAlbumFolder, 
    openLockFile, 
    openManifestFile, 
    updateAlbum 
  } from "../../api.ts";

  let { hasLyrics, hasPalette }: { hasLyrics: boolean, hasPalette: boolean } = $props();

  let activeId = $derived(player.currentAlbumId);
  let isStopped = $derived(player.state === "stop");

  async function handlePlay() {
    if (activeId) await playAlbum(activeId);
  }

  async function handleOpenFolder() {
    if (activeId) await openAlbumFolder(activeId);
  }

  async function handleOpenLock() {
    if (activeId) await openLockFile(activeId);
  }

  async function handleOpenManifest() {
    if (activeId) await openManifestFile(activeId);
  }

  async function handleUpdate() {
    if (activeId) await updateAlbum(activeId);
  }
</script>

{#snippet NavButton({ icon, label, disabled, active, onclick }: { icon: string, label: string, disabled?: boolean, active?: boolean, onclick: () => void })}
  <button class="v-btn-icon queue-nav-button" class:active {disabled} {onclick} title={label}>
    <img src="/{icon}" alt={label} class="nav-icon" />
  </button>
{/snippet}

{#snippet ActButton({ icon, label, disabled, active, onclick }: { icon: string, label: string, disabled?: boolean, active?: boolean, onclick: () => void })}
  <button class="v-btn-icon queue-act-button" class:active {disabled} {onclick} title={label}>
    <img src="/{icon}" alt={label} class="act-icon" />
  </button>
{/snippet}

<div class="queue-bar v-glass">
  <div class="nav-group top">
    {@render ActButton({ icon: "icons/outlined/24px/code.svg", label: "Open Data Object", disabled: !activeId, onclick: handleOpenLock })}
    {@render ActButton({ icon: "icons/outlined/24px/edit_document.svg", label: "Open Manifest", disabled: !activeId, onclick: handleOpenManifest })}
    {@render ActButton({ icon: "icons/outlined/24px/folder.svg", label: "Open Local Folder", disabled: !activeId, onclick: handleOpenFolder })}
    {@render ActButton({ icon: "icons/outlined/24px/refresh.svg", label: "Update Album", disabled: !activeId, onclick: handleUpdate })}
  </div>

  <div class="nav-group bottom">
    {#if hasLyrics}
      {@render NavButton({ icon: "icons/outlined/24px/menu_book.svg", label: "Lyrics", active: library.queuePanels.lyrics, onclick: () => library.toggleQueuePanel('lyrics') })}
    {/if}
    {@render NavButton({ icon: "icons/outlined/24px/format_list_bulleted.svg", label: "Track List", active: library.queuePanels.tracks, onclick: () => library.toggleQueuePanel('tracks') })}
    {#if hasPalette}
      {@render NavButton({ 
        icon: "icons/outlined/24px/colors.svg", 
        label: "Toggle Shader", 
        active: library.isShaderEnabled, 
        disabled: isStopped,
        onclick: () => library.toggleShader() 
      })}
    {/if}
  </div>
</div>

<style>
  .queue-bar {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    padding: 10px;
    box-sizing: border-box;
    z-index: 100;
    flex-shrink: 0;
  }
  
  .nav-group {
    display: flex;
    flex-direction: column;
    gap: 10px;
  }

  .queue-nav-button {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    box-shadow: var(--button-shadow-lesser);
    flex-shrink: 0;
    pointer-events: auto;
  }

  .queue-nav-button:disabled {
    opacity: 0.3;
    pointer-events: none;
    box-shadow: none;
  }

  .queue-act-button {
    width: 36px;
    height: 36px;
    border-radius: 20px;
    box-shadow: var(--button-shadow-lesser);
    flex-shrink: 0;
    pointer-events: auto;
  }

  .queue-act-button:disabled {
    opacity: 0.3;
    pointer-events: none;
    box-shadow: none;
  }

  .nav-icon {
    width: 20px;
    height: 22px;
  }

  .act-icon {
    width: 18px;
    height: 18px;
  }
</style>

