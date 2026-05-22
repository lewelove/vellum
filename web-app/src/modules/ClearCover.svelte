<script lang="ts">
  let { hash, width, height, animate = true }: { hash?: string, width: number, height: number, animate?: boolean } = $props();

  let dpr = $derived(window.devicePixelRatio || 1);
  let targetWidth = $derived(Math.round(width * dpr));

  let srcUrl = $derived(hash && targetWidth > 0 ? `/api/resize/${targetWidth}px/${hash}?v=${hash}` : "");

  let isLoaded = $state(false);
  let blobUrl = $state("");
  let apiCallStart = 0;

  $effect(() => {
    if (!srcUrl) {
      blobUrl = "";
      isLoaded = false;
      return;
    }

    let active = true;
    let localBlobUrl = "";
    apiCallStart = performance.now();
    console.log(`[Cover API] Call initiated for ${targetWidth}px at ${apiCallStart.toFixed(2)}ms`);

    fetch(srcUrl)
      .then(res => {
        if (!res.ok) throw new Error("Fetch failed");
        return res.blob();
      })
      .then(blob => {
        if (!active) return;
        const coverReceived = performance.now();
        console.log(`[Cover API] Cover bytes received in ${(coverReceived - apiCallStart).toFixed(2)}ms`);
        
        localBlobUrl = URL.createObjectURL(blob);
        blobUrl = localBlobUrl;
      })
      .catch(err => {
        console.error(err);
      });

    return () => {
      active = false;
      isLoaded = false;
      if (localBlobUrl) {
        URL.revokeObjectURL(localBlobUrl);
      }
    };
  });

  function handleLoad() {
    isLoaded = true;
    const renderReady = performance.now();
    console.log(`[Cover API] Cover fully ready to render. Total pipeline: ${(renderReady - apiCallStart).toFixed(2)}ms`);
  }
</script>

<div class="clear-cover-wrapper" style="width: {width}px; height: {height}px;">
  {#if blobUrl}
    <div 
      class="cover-block" 
      class:visible={isLoaded || !animate}
      style={animate ? "" : "transition: none; will-change: auto;"}
    >
      <img
        src={blobUrl}
        class="cover-image"
        alt=""
        draggable="false"
        onload={handleLoad}
      />
    </div>
  {:else}
    <div class="empty-cover">
    </div>
  {/if}
</div>

<style>
  .clear-cover-wrapper {
    position: relative;
    overflow: visible;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .cover-block {
    position: absolute;
    inset: 0;
    opacity: 0;
    transition: opacity 0.2s ease;
    will-change: opacity 0.2s ease;
  }

  .cover-block.visible {
    opacity: 1;
  }

  .cover-image {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
    box-shadow: var(--album-cover-shadow);
  }

  .empty-cover {
    width: 100%;
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    box-sizing: border-box;
  }
</style>
