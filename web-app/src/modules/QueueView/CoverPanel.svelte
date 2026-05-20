<script lang="ts">
  import { player } from "../player.svelte.ts";
  import ClearCover from "../ClearCover.svelte";

  let { coverHash = "", onclick, width = $bindable(0) }: { coverHash?: string, onclick?: () => void, width?: number } = $props();
  let isStopped = $derived(player.state === "stop");
</script>

<div 
  class="cover-wrapper" 
  class:stopped={isStopped}
  style="background-color: {isStopped ? 'var(--background-drawer)' : 'transparent'};"
>
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
    height: 100%;
    max-height: 100%;
    max-width: 60vw;
    aspect-ratio: 1 / 1;
    display: flex;
    justify-content: center;
    align-items: center;
    position: relative;
    z-index: 10;
    transition: background-color 0.3s ease;
    box-sizing: border-box;
  }

  .cover-wrapper.stopped {
    box-shadow: var(--album-cover-shadow);
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
