<script lang="ts">
  import { player } from "../player.svelte.ts";
  import ClearCover from "../ClearCover.svelte";

  let { coverHash = "", onclick, width = $bindable(0) }: { coverHash?: string, onclick?: () => void, width?: number } = $props();
</script>

<div class="cover-wrapper v-glass">
  <button 
    class="cover-panel" 
    class:clickable={!!coverHash}
    bind:clientWidth={width}
    onclick={coverHash ? onclick : undefined}
    type="button"
    aria-label={coverHash ? "Expand cover" : "Cover art"}
  >
    <div class="cover-absolute-wrapper">
      {#if width > 0}
        <ClearCover 
          hash={coverHash} 
          width={width} 
          height={width} 
        />
      {/if}
    </div>
  </button>
</div>

<style>
  .cover-wrapper {
    height: 100%;
    max-height: 100%;
    max-width: 60vw;
    display: flex;
    justify-content: center;
    align-items: center;
    position: relative;
    z-index: 10;
    transition: background-color 0.3s ease;
    box-sizing: border-box;
    padding: 20px 0;
    box-shadow: none;
  }

  .cover-panel {
    height: 100%;
    aspect-ratio: 1 / 1;
    position: relative;
    cursor: default;
    outline: none;
    border: none;
    box-sizing: border-box;
    background: none;
    padding: 0;
  }

  .cover-panel.clickable {
    cursor: pointer;
  }

  .cover-absolute-wrapper {
    position: absolute;
    inset: 0;
    display: flex;
    align-items: center;
    justify-content: center;
  }
</style>
