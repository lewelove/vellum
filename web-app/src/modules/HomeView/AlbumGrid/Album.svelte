<script lang="ts">
  import { theme } from "../../../theme.svelte.ts";
  import { library } from "../../../library.svelte.ts";

  let { album, active, onclick, scrollY = 0, rowY = 0 }: { album: any, active: boolean, onclick: () => void, scrollY?: number, rowY?: number } = $props();

  let originalUrl = $derived(library.getThumbnailUrl(album));
  let coverBitmap = $derived(library.pinnedTextures.get(originalUrl));

  const coverSize = $derived(theme.albumGrid["cover-size"]);
  const gapY = $derived(theme.albumGrid["gap-y"]);
  const textGap = $derived(theme.albumGrid["text-gap-main"]);

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
  
  const lhTitle = $derived(theme.albumGrid["font-line-height-title"]);
  const gapLesser = $derived(theme.albumGrid["text-gap-lesser"]);
  const lhArtist = $derived(theme.albumGrid["font-line-height-artist"]);
  const textBlockHeight = $derived(lhTitle + gapLesser + lhArtist);
  
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
    if (!canvas) return;
    
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
    
    const bgHex = "#333333";
    ctx.fillStyle = bgHex;
    ctx.fillRect(0, -1, w, h + 2);
    
    const fontStack = "Inter Vellum, 'Noto Sans', system-ui, sans-serif";
    
    applyEffects(ctx);

    const palette = theme.palette as Record<string, string>;
    const colors = theme.colors as Record<string, string>;

    const cTitle = palette[colors["text-main"]] || "#ffffff";
    const sTitle = theme.typography["font-size-title"];
    const wTitle = theme.typography["font-weight-title"];
    
    ctx.fillStyle = cTitle;
    ctx.font = `${wTitle} ${sTitle}px ${fontStack}`;
    ctx.textBaseline = "middle"; 
    
    const titleY = lhTitle / 2;
    const titleText = fitText(ctx, album.title, w);
    ctx.fillText(titleText, 0, titleY);
    
    const cArtist = palette[colors["text-muted"]] || "#cccccc";
    const sArtist = theme.typography["font-size-artist"];
    const wArtist = theme.typography["font-weight-artist"];
    
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
      c1: theme.colors["text-main"],
      c2: theme.colors["text-muted"],
      bg: theme.colors["background-main"],
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

<div class="album-unit">
  <button 
    class="album-cover" 
    class:active
    style="z-index: 10;"
    {onclick}
    aria-label="Select album {album.title}"
  >
    {#if originalUrl}
      <img 
        src={originalUrl} 
        alt="" 
        decoding="sync"
        draggable="false"
        style="opacity: {coverBitmap ? 0 : 1}; position: absolute; inset: 0;"
      />
    {/if}
    <canvas 
        bind:this={coverCanvas}
        style="opacity: {coverBitmap ? 1 : 0}; width: 100%; height: calc(100% + 2px); position: absolute; top: -1px; left: 0; display: block; pointer-events: none;"
    ></canvas>
  </button>
  
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
</div>

<style>
  .album-unit {
    display: flex;
    flex-direction: column;
    flex-shrink: 0; 
    width: var(--cover-size);
    padding-top: var(--gap-y);
    position: relative;
  }

  .album-cover {
    border: none;
    padding: 0;
    cursor: pointer;
    display: block;
    outline: none !important;
    width: var(--cover-size);
    height: var(--cover-size);
    margin-bottom: var(--text-gap-main);
    position: relative;
    background-color: #323232;
    border-radius: 0px;
    box-shadow: var(--album-cover-shadow);
    transition: transform 0.2s ease, box-shadow 0.2s ease;
    pointer-events: auto;
    overflow: visible;
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
