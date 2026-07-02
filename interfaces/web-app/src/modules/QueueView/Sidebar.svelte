<script lang="ts">
  import { view } from "../../library/view.svelte.ts";
  import { player } from "../player.svelte.ts";
  import { setTab } from "../../navigation.svelte.ts";

  let { hasPalette }: { hasPalette: boolean } = $props();

  let activeId = $derived(player.currentAlbumId);
  let isStopped = $derived(player.state === "stop");

  async function handleFocus() {
    if (activeId) {
      await setTab("home");
      await view.setFocus({ id: activeId });
    }
  }
</script>

{#snippet NavButton({ icon, label, disabled, active, onclick }: { icon: string, label: string, disabled?: boolean, active?: boolean, onclick: () => void })}
  <button class="v-btn-icon queue-nav-button" class:active {disabled} {onclick} title={label}>
    <img src="/{icon}" alt={label} class="nav-icon" />
  </button>
{/snippet}

<div class="queue-bar v-glass">
  <div class="nav-group bottom">
    {@render NavButton({ icon: "icons/outlined/24px/menu_book.svg", label: "Control", active: view.queuePanels.control, onclick: () => view.toggleQueuePanel('control') })}
    {@render NavButton({ icon: "icons/outlined/24px/format_list_bulleted.svg", label: "Track List", active: view.queuePanels.tracks, onclick: () => view.toggleQueuePanel('tracks') })}
    {#if hasPalette}
      {@render NavButton({ 
        icon: "icons/outlined/24px/colors.svg", 
        label: "Toggle Shader", 
        active: view.isShaderEnabled, 
        disabled: isStopped,
        onclick: () => view.toggleShader() 
      })}
    {/if}
    {@render NavButton({
      icon: "icons/outlined/24px/album.svg",
      label: "Focus Album",
      disabled: !activeId,
      onclick: handleFocus
    })}
  </div>
</div>

<style>
  .queue-bar {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    align-items: center;
    padding: 8px;
    gap: 10px;
    box-sizing: border-box;
    z-index: 100;
    flex-shrink: 0;
  }
  
  .nav-group {
    display: flex;
    flex-direction: column;
    gap: 8px;
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

  .nav-icon {
    width: 20px;
    height: 20px;
  }
</style>
