<script lang="ts">
  import { config } from "../../../config.svelte.ts";
  import { collection } from "../../../library/collection.svelte.ts";
  import { prewarmer } from "../../../library/prewarmer.svelte.ts";

  let { album, active, onclick, scrollY = 0, rowY = 0 }: { album: any, active: boolean, onclick: () => void, scrollY?: number, rowY?: number } = $props();

  let originalUrl = $derived(collection.getThumbnailUrl(album));
  let coverBitmap = $derived(prewarmer.pinnedTextures.get(originalUrl));

  const coverSize = $derived(config.album_grid.album_card.cover.size);
  const gapY = $derived(config.album_grid.spacing.y);
  const textGap = $derived(config.album_grid.album_card.text.enable ? config.album_grid.album_card.text.spacing.top : 0);

  let absoluteY = $derived(rowY - scrollY);
  let metadataTop = $derived(absoluteY + gapY + coverSize + textGap);
  
  let opacity = $derived.by(() => {
    const fadeDistance = 40;
    const diff = metadataTop; 
    
    if (diff >= fadeDistance) return 1;
    if (diff <= 0) return 0;
    
    return diff / fadeDistance;
  });

  let canvas: HTMLCanvasElement | undefined = $state();
  let coverCanvas: HTMLCanvasElement | undefined = $state();
  
  const lhTitle = $derived(Math.round(config.album_grid.album_card.text.title.size * 1.2));
  const gapLesser = $derived(config.album_grid.album_card.text.spacing.middle);
  const lhArtist = $derived(Math.round(config.album_grid.album_card.text.albumartist.size * 1.2));
  const textBlockHeight = $derived(config.album_grid.album_card.text.enable ? (lhTitle + gapLesser + lhArtist) : 0);
  
  function fitText(ctx: CanvasRenderingContext2D, text: string, maxWidth: number) {
    if (!text) return "";
    let ellipsis = "...";
    let width = ctx.measureText(text).width;
    if (width <= maxWidth) return text;
    
    let len = text.length;
    while (width > maxWidth && len > 0) {
      len--;
      width = ctx.measureText(text.substring(0, len) + ellipsis).width;
    }
    return text.substring(0, len) + ellipsis;
  }

  function applyEffects(ctx: CanvasRenderingContext2D) {
    ctx.shadowColor = "rgba(0, 0, 0, 0.1)";
    ctx.shadowBlur = 4;
    ctx.shadowOffsetX = 0;
    ctx.shadowOffsetY = 0;
  }

  function renderText() {
    if (!canvas || !config.album_grid.album_card.text.enable) return;
    
    const dpr = window.devicePixelRatio || 1;
    const w = coverSize;
    const h = textBlockHeight;

    if (w <= 0 || h <= 0) return;

    canvas.width = w * dpr;
    canvas.height = (h + 2) * dpr;
    
    const ctx = canvas.getContext('2d', { alpha: false });
    if (!ctx) return;
    ctx.scale(dpr, dpr);
    ctx.translate(0, 1);
    
    const bgHex = config.palette["200"] || "#323232";
    ctx.fillStyle = bgHex;
    ctx.fillRect(0, -1, w, h + 2);
    
    const fontStack = "Inter Vellum, 'Noto Sans', system-ui, sans-serif";
    
    applyEffects(ctx);

    const cTitle = config.palette["500"] || "#ffffff";
    const sTitle = config.album_grid.album_card.text.title.size;
    const wTitle = 400;
    
    ctx.fillStyle = cTitle;
    ctx.font = `${wTitle} ${sTitle}px ${fontStack}`;
    ctx.textBaseline = "middle"; 
    
    const titleY = lhTitle / 2;
    const titleText = fitText(ctx, album.title, w);
    ctx.fillText(titleText, 0, titleY);
    
    const cArtist = config.palette["400"] || "#cccccc";
    const sArtist = config.album_grid.album_card.text.albumartist.size;
    const wArtist = 400;
    
    ctx.fillStyle = cArtist;
    ctx.font = `${wArtist} ${sArtist}px ${fontStack}`;
    
    const artistY = lhTitle + gapLesser + (lhArtist / 2);
    const artistText = fitText(ctx, album.artist, w);
    ctx.fillText(artistText, 0, artistY);
  }

  $effect(() => {
    const _ = {
      t: album.title, 
      a: album.artist, 
      w: coverSize, 
      h: textBlockHeight,
      p500: config.palette["500"],
      p400: config.palette["400"],
      p200: config.palette["200"],
      dpr: window.devicePixelRatio 
    };
    renderText();
  });

  $effect(() => {
    if (coverCanvas && coverBitmap) {
      const ctx = coverCanvas.getContext('2d', { alpha: false });
      if (!ctx) return;
      const dpr = window.devicePixelRatio || 1;
      coverCanvas.width = coverSize * dpr;
      coverCanvas.height = (coverSize + 2) * dpr;
      
      ctx.scale(dpr, dpr);
      ctx.translate(0, 1);
      
      const bgHex = "#292929";
      ctx.fillStyle = bgHex;
      ctx.fillRect(0, -1, coverSize, coverSize + 2);
      
      ctx.drawImage(coverBitmap, 0, 0, coverSize, coverSize);
    }
  });
</script>

<div class="album-unit" style="width: {coverSize}px; padding-top: {gapY}px;">
  <button 
    class="album-cover" 
    class:active
    style="z-index: 10; width: {coverSize}px; height: {coverSize}px; margin-bottom: {textGap}px;"
    {onclick}
    aria-label="Select album {album.title}"
  >
    {#if originalUrl}
      <img 
        src={originalUrl} 
        alt="" 
        decoding="sync"
        draggable="false"
        style="opacity: {coverBitmap ? 0 : 1}; position: absolute; inset: 0; width: 100%; height: 100%; z-index: 1;"
      />
    {/if}
    <canvas 
        bind:this={coverCanvas}
        style="opacity: {coverBitmap ? 1 : 0}; width: 100%; height: calc(100% + 2px); position: absolute; top: -1px; left: 0; display: block; pointer-events: none; z-index: 2;"
    ></canvas>
    <div 
      class="cover-shadow-overlay"
      style="position: absolute; top: -1px; left: 0; width: 100%; height: calc(100% + 2px); pointer-events: none; z-index: 3;"
    ></div>
  </button>
  
  {#if config.album_grid.album_card.text.enable}
    <div 
      class="album-info" 
      style="
        opacity: {opacity};
        z-index: 1;
        height: {textBlockHeight}px;
      "
    >
      <canvas 
        bind:this={canvas}
        style="
          width: {coverSize}px;
          height: calc(100% + 2px);
          position: absolute;
          top: -1px;
          left: 0;
          display: block;
          image-rendering: -webkit-optimize-contrast;
        "
      ></canvas>
    </div>
  {/if}
</div>

<style>
  .album-unit {
    display: flex;
    flex-direction: column;
    flex-shrink: 0; 
    position: relative;
  }

  .album-cover {
    border: none;
    padding: 0;
    cursor: pointer;
    display: block;
    outline: none !important;
    position: relative;
    border-radius: 0px;
    transition: transform 0.2s ease;
    pointer-events: auto;
    overflow: visible;
  }

  .cover-shadow-overlay {
    box-shadow: var(--album-cover-shadow);
    transition: box-shadow 0.2s ease;
  }

  .album-cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
    display: block;
  }

  .album-info {
    display: block;
    position: relative;
    will-change: opacity;
  }
</style>
