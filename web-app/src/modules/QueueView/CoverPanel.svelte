<script lang="ts">
  import { player } from "../player.svelte.ts";
  import ClearCover from "../ClearCover.svelte";

  let { coverHash = "", onclick, width = $bindable(0) }: { coverHash?: string, onclick?: () => void, width?: number } = $props();
  let isStopped = $derived(player.state === "stop");
</script>

<div class="cover-wrapper" class:v-glass={isStopped}>
  <div 
    class="cover-panel" 
    class:clickable={!!coverHash}
    bind:clientWidth={width}
    {onclick}
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
  </div>
</div>

<style>
  .cover-wrapper {
    background-color: #1f1f1f;
    flex: 0 1 auto;
    height: 100%;
    max-height: 100%;
    max-width: 60%;
    aspect-ratio: 1 / 1;
    align-self: flex-start;
    min-width: 0;
    min-height: 0;
    display: flex;
    justify-content: center;
    align-items: center;
    position: relative;
    z-index: 10;
  }

  .cover-panel {
    width: 100%;
    height: 100%;
    position: relative;
    cursor: default;
    outline: none;
    border: none;
    box-sizing: border-box;
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
