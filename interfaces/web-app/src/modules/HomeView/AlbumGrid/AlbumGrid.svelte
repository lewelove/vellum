<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { GridController } from "./GridController.svelte.ts";
  import Album from "./Album.svelte";

  let { albums = [], version = 0, activeAlbumId = null, onfocus = () => {} }: { albums?: any[], version?: number, activeAlbumId?: string | null, onfocus?: (album: any) => void } = $props();

  const ctrl = new GridController(() => albums);
  let rafId: number;
  let dpr = $state(1);

  const activeKeys = new Set();
  const SCROLL_SPEED = 0.20;
  let isAnimating = false;

  let renderY = $derived(ctrl.scroll.currentY);

  function loop() {
    let delta = 0;
    if (activeKeys.has('j') || activeKeys.has('arrowdown')) delta += SCROLL_SPEED;
    if (activeKeys.has('k') || activeKeys.has('arrowup')) delta -= SCROLL_SPEED;

    if (delta !== 0) ctrl.scrollRow(delta);

    const idealTargetY = ctrl.scroll.targetSlot * ctrl.layout.rowHeight;
    const snappedTargetY = Math.round(idealTargetY * dpr) / dpr;
    const diff = Math.abs(snappedTargetY - ctrl.scroll.currentY);

    if (delta !== 0 || diff > 0.01) {
      ctrl.update(null, dpr);
      rafId = requestAnimationFrame(loop);
    } else {
      ctrl.update(null, dpr);
      isAnimating = false;
    }
  }

  function wakeUp() {
    if (!isAnimating) {
      isAnimating = true;
      loop();
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName ?? "")) return;
    if (activeAlbumId) return;

    const key = e.key.toLowerCase();
    if (['j', 'k', 'arrowdown', 'arrowup'].includes(key)) {
      e.preventDefault();
      activeKeys.add(key);
      wakeUp();
    }
  }

  function handleKeyup(e: KeyboardEvent) {
    const key = e.key.toLowerCase();
    if (activeKeys.has(key)) activeKeys.delete(key);
  }

  function handleBlur() {
    activeKeys.clear();
  }

  let prevCols = 0;
  $effect(() => {
    if (ctrl.layout.cols !== prevCols && prevCols !== 0) {
      const topAlbumIdx = ctrl.scroll.targetSlot * prevCols;
      const newSlot = Math.floor(topAlbumIdx / ctrl.layout.cols);
      ctrl.scroll.syncToSlot(newSlot);
      wakeUp();
    }
    prevCols = ctrl.layout.cols;
  });

  $effect(() => {
    const _v = version;
    ctrl.resetScroll();
    wakeUp();
  });

  onMount(() => {
    dpr = window.devicePixelRatio || 1;
    window.addEventListener("keydown", handleKeydown);
    window.addEventListener("keyup", handleKeyup);
    window.addEventListener("blur", handleBlur);
    wakeUp();
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
    window.removeEventListener("keyup", handleKeyup);
    window.removeEventListener("blur", handleBlur);
    if (rafId) cancelAnimationFrame(rafId);
  });
</script>

<div class="album-grid-viewport">
  <div 
    class="grid-container"
    bind:clientWidth={ctrl.layout.containerWidth} 
    bind:clientHeight={ctrl.viewportHeight}
    onwheel={(e) => { 
      if (activeAlbumId) return;
      e.preventDefault(); 
      ctrl.handleWheel(e); 
      wakeUp();
    }}
  >
    <div 
      class="scroll-content" 
      style="
        height: {ctrl.contentHeight}px; 
        background-color: var(--background-main);
        transform: translate3d(0, -{renderY}px, 0);
        will-change: transform;
      "
    >
      {#each ctrl.virtualRows as row (row.index)}
        <div 
          class="row" 
          style="
            transform: translateY({row.y}px); 
            width: {ctrl.layout.gridWidth}px; 
            height: {ctrl.layout.rowHeight}px;
          "
        >
          <div class="row-inner" style="gap: var(--gap-x);">
              {#each row.data as album (album.id)}
                <Album 
                  {album} 
                  active={activeAlbumId === album.id}
                  onclick={() => onfocus(album)} 
                  scrollY={renderY}
                  rowY={row.y}
                />
              {/each}
          </div>
        </div>
      {/each}
    </div>
  </div>
</div>

<style>
    .album-grid-viewport {
      position: relative;
      width: 100%;
      height: 100%;
      overflow: hidden;
    }

    .grid-container {
      width: 100%;
      height: 100%;
      position: relative;
      overflow: hidden;
      overscroll-behavior: none;
      contain: content;
    }

    .scroll-content {
      width: 100%;
      position: absolute;
      top: 0;
      left: 0;
      pointer-events: auto;
      backface-visibility: hidden;
      transform-style: preserve-3d;
    }
    
    .row {
        position: absolute;
        margin: 0 auto;
        right: 0;
        left: 0;
        display: flex;
        flex-direction: column;
        overflow: visible; 
        will-change: transform;
        backface-visibility: hidden;
    }
    
    .row-inner {
        display: flex;
        justify-content: flex-start;
        height: 100%;
    }
</style>
