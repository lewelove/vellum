<script lang="ts">
  let { hash, width, height, animate = true }: { hash?: string, width: number, height: number, animate?: boolean } = $props();

  let dpr = $derived(window.devicePixelRatio || 1);
  let targetWidth = $derived(Math.round(width * dpr));

  let algo = 'mitchell';
  let srcUrl = $derived(hash && targetWidth > 0 ? `/api/covers/${algo}/${targetWidth}px/${hash}?v=${hash}` : "");
  let thumbUrl = $derived(hash ? `/api/covers/lanczos/200px/${hash}?v=${hash}` : "");

  let isLoaded = $state(false);

  $effect(() => {
    if (srcUrl) {
      isLoaded = false;
    }
  });
</script>

<div class="clear-cover-wrapper" style="width: {width}px; height: {height}px;">
  {#if hash}
    {#key hash}
      <img
        src={thumbUrl}
        class="cover-image placeholder"
        alt=""
        draggable="false"
      />
      {#if srcUrl}
        <img
          src={srcUrl}
          class="cover-image high-res"
          class:visible={isLoaded || !animate}
          style={animate ? "" : "transition: none; will-change: auto;"}
          alt=""
          draggable="false"
          onload={() => isLoaded = true}
        />
      {/if}
    {/key}
  {:else}
    <div class="empty-cover"></div>
  {/if}
</div>

<style>
  .clear-cover-wrapper {
    position: relative;
    overflow: visible;
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: var(--album-cover-shadow);
  }

  .cover-image {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .placeholder {
    z-index: 1;
  }

  .high-res {
    z-index: 2;
    opacity: 0;
    transition: opacity 0.2s ease;
    will-change: opacity;
  }

  .high-res.visible {
    opacity: 1;
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
