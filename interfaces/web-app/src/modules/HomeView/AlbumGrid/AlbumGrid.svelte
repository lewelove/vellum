<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { GridController } from "./GridController.svelte.ts";
  import { config } from "../../../config.svelte.ts";
  import { collection } from "../../../library/collection.svelte.ts";
  import { prewarmer } from "../../../library/prewarmer.svelte.ts";

  let { albums = [], version = 0, activeAlbumId = null, onfocus = () => {} }: { albums?: any[], version?: number, activeAlbumId?: string | null, onfocus?: (album: any) => void } = $props();

  const ctrl = new GridController();
  
  let rafId: number | null = null;
  let dpr = 1;

  let canvasEl: HTMLCanvasElement | undefined;
  let ctx: CanvasRenderingContext2D | null = null;

  const activeKeys = new Set();
  const SCROLL_SPEED = 0.20;
  let isAnimating = false;

  let hoveredAlbum: any = null;
  let textCache = new Map();
  let emptyCoverCanvas: any = null;

  function fitText(cCtx: CanvasRenderingContext2D, text: string, maxWidth: number) {
    if (!text) return "";
    let ellipsis = "...";
    let width = cCtx.measureText(text).width;
    if (width <= maxWidth) return text;
    
    let len = text.length;
    while (width > maxWidth && len > 0) {
      len--;
      width = cCtx.measureText(text.substring(0, len) + ellipsis).width;
    }
    return text.substring(0, len) + ellipsis;
  }

  function getTextCanvas(album: any, coverSize: number, textBlockHeight: number, textConfig: any) {
    if (textBlockHeight <= 0) return null;
    if (textCache.has(album.id)) return textCache.get(album.id);
    
    const c = document.createElement('canvas');
    c.width = coverSize * dpr;
    c.height = textBlockHeight * dpr;
    const cCtx = c.getContext('2d', { alpha: false });
    if (!cCtx) return c;
    
    cCtx.scale(dpr, dpr);
    
    const bgHex = config.palette["200"] || "#323232";
    cCtx.fillStyle = bgHex;
    cCtx.fillRect(0, 0, coverSize, textBlockHeight);

    cCtx.shadowColor = "rgba(0, 0, 0, 0.1)";
    cCtx.shadowBlur = 4;
    cCtx.shadowOffsetX = 0;
    cCtx.shadowOffsetY = 0;

    const fontStack = "'Inter Vellum', 'Noto Sans', system-ui, sans-serif";
    
    const cTitle = config.palette["500"] || "#ffffff";
    const sTitle = textConfig.title.size;
    const lhTitle = Math.round(sTitle * 1.2);
    cCtx.fillStyle = cTitle;
    cCtx.font = `400 ${sTitle}px ${fontStack}`;
    cCtx.textBaseline = "middle";
    const titleText = fitText(cCtx, album.title, coverSize);
    cCtx.fillText(titleText, 0, lhTitle / 2);

    const cArtist = config.palette["400"] || "#cccccc";
    const sArtist = textConfig.albumartist.size;
    const gapLesser = textConfig.spacing.middle;
    const lhArtist = Math.round(sArtist * 1.2);
    cCtx.fillStyle = cArtist;
    cCtx.font = `400 ${sArtist}px ${fontStack}`;
    const artistY = lhTitle + gapLesser + (lhArtist / 2);
    const artistText = fitText(cCtx, album.artist, coverSize);
    cCtx.fillText(artistText, 0, artistY);

    textCache.set(album.id, c);
    return c;
  }

  function getEmptyCoverCanvas(size: number) {
    if (emptyCoverCanvas && emptyCoverCanvas.size === size && emptyCoverCanvas.dpr === dpr) return emptyCoverCanvas;
    
    const pad = 24;
    const c = document.createElement('canvas');
    c.width = (size + pad * 2) * dpr;
    c.height = (size + pad * 2) * dpr;
    const cCtx = c.getContext('2d');
    if (!cCtx) return { canvas: c, size, dpr, pad };
    
    cCtx.scale(dpr, dpr);
    
    cCtx.shadowColor = "rgba(0, 0, 0, 0.3)";
    cCtx.shadowBlur = 16;
    cCtx.shadowOffsetX = 0;
    cCtx.shadowOffsetY = 0;
    cCtx.fillStyle = "#292929";
    cCtx.fillRect(pad, pad, size, size);

    cCtx.shadowColor = "rgba(0, 0, 0, 0.1)";
    cCtx.shadowBlur = 12;
    cCtx.fillRect(pad, pad, size, size);
    
    cCtx.shadowBlur = 6;
    cCtx.fillRect(pad, pad, size, size);

    emptyCoverCanvas = { canvas: c, size, dpr, pad };
    return emptyCoverCanvas;
  }

  function renderCanvas() {
    if (!canvasEl) return;
    if (!ctx) {
        ctx = canvasEl.getContext('2d', { alpha: false });
        if (!ctx) return;
    }
    const w = ctrl.layout.containerWidth;
    const h = ctrl.viewportHeight;
    if (w === 0 || h === 0) return;

    const currentDpr = window.devicePixelRatio || 1;
    if (dpr !== currentDpr || canvasEl.width !== Math.floor(w * currentDpr) || canvasEl.height !== Math.floor(h * currentDpr)) {
      dpr = currentDpr;
      canvasEl.width = Math.floor(w * dpr);
      canvasEl.height = Math.floor(h * dpr);
      emptyCoverCanvas = null;
      textCache.clear();
    }

    ctx.save();
    ctx.scale(dpr, dpr);
    
    ctx.fillStyle = config.palette["200"] || "#323232";
    ctx.fillRect(0, 0, w, h);

    const coverSize = ctrl.layout.cardSize;
    const gapX = ctrl.layout.gapX;
    const gapY = ctrl.layout.gapY;
    const gridWidth = ctrl.layout.gridWidth;
    const startX = Math.floor((w - gridWidth) / 2);
    
    const textConfig = config.album_grid.album_card.text;
    const textGap = textConfig.enable ? textConfig.spacing.top : 0;
    const lhTitle = Math.round(textConfig.title.size * 1.2);
    const lhArtist = Math.round(textConfig.albumartist.size * 1.2);
    const textBlockHeight = textConfig.enable ? (lhTitle + textConfig.spacing.middle + lhArtist) : 0;
    
    const shadowTile = getEmptyCoverCanvas(coverSize);
    
    const scrollY = ctrl.scroll.currentY;
    const rows = ctrl.virtualRows;

    for (const row of rows) {
      if (!row || !row.data) continue;

      const rowY = row.y - scrollY;
      if (rowY + ctrl.layout.rowHeight < -100 || rowY > h + 100) continue; 

      for (let i = 0; i < row.data.length; i++) {
        const album = row.data[i];
        const x = startX + i * (coverSize + gapX);
        const y = rowY + gapY;

        ctx.drawImage(shadowTile.canvas, 0, 0, shadowTile.canvas.width, shadowTile.canvas.height, x - shadowTile.pad, y - shadowTile.pad, coverSize + shadowTile.pad*2, coverSize + shadowTile.pad*2);

        const url = collection.getThumbnailUrl(album);
        const bitmap = prewarmer.pinnedTextures.get(url);
        
        if (bitmap) {
          ctx.drawImage(bitmap, x, y, coverSize, coverSize);
        } else {
          prewarmer.loadNow(url);
        }

        if (textConfig.enable && textBlockHeight > 0) {
          const absoluteY = row.y - scrollY;
          const metadataTop = absoluteY + gapY + coverSize + textGap;
          const fadeDistance = 40;
          let opacity = 1;
          if (metadataTop < fadeDistance) {
            opacity = Math.max(0, metadataTop / fadeDistance);
          }

          if (opacity > 0) {
            ctx.globalAlpha = opacity;
            const tCanvas = getTextCanvas(album, coverSize, textBlockHeight, textConfig);
            if (tCanvas) {
              ctx.drawImage(tCanvas, 0, 0, tCanvas.width, tCanvas.height, x, y + coverSize + textGap, coverSize, textBlockHeight);
            }
            ctx.globalAlpha = 1.0;
          }
        }
      }
    }
    ctx.restore();
  }

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
      renderCanvas();
      rafId = requestAnimationFrame(loop);
    } else {
      ctrl.update(null, dpr);
      renderCanvas();
      isAnimating = false;
      rafId = null;
    }
  }

  function wakeUp() {
    if (!isAnimating) {
      isAnimating = true;
      rafId = requestAnimationFrame(loop);
    } else if (!rafId) {
      rafId = requestAnimationFrame(loop);
    }
  }

  function getAlbumAt(clientX: number, clientY: number) {
    const { gapX, gapY, cardSize, rowHeight, gridWidth, cols, topOffset } = ctrl.layout;
    const w = ctrl.layout.containerWidth;
    const startX = Math.floor((w - gridWidth) / 2);

    const localGridX = clientX - startX;
    if (localGridX < 0 || localGridX > gridWidth) return null;

    const colIndex = Math.floor(localGridX / (cardSize + gapX));
    const localX = localGridX - colIndex * (cardSize + gapX);
    if (localX > cardSize) return null;

    const scrollY = ctrl.scroll.currentY;
    const absoluteY = clientY + scrollY;

    const adjustedY = absoluteY - topOffset;
    if (adjustedY < 0) return null;

    const rowIndex = Math.floor(adjustedY / rowHeight);
    const rowYPos = rowIndex * rowHeight + topOffset;
    const localY = absoluteY - rowYPos;

    if (localY < gapY || localY > gapY + cardSize) return null; 

    if (rowIndex < ctrl.allRows.length && colIndex < cols) {
      const row = ctrl.allRows[rowIndex];
      if (row && colIndex < row.length) {
        return row[colIndex];
      }
    }
    return null;
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

  function handlePointerMove(e: PointerEvent) {
    if (activeAlbumId || !canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    const hovered = getAlbumAt(x, y);
    if (hovered !== hoveredAlbum) {
      hoveredAlbum = hovered;
      canvasEl.style.cursor = hovered ? 'pointer' : 'default';
    }
  }

  function handlePointerLeave() {
    if (hoveredAlbum !== null && canvasEl) {
      hoveredAlbum = null;
      canvasEl.style.cursor = 'default';
    }
  }

  function handleClick(e: MouseEvent) {
    if (activeAlbumId || !canvasEl) return;
    const rect = canvasEl.getBoundingClientRect();
    const x = e.clientX - rect.left;
    const y = e.clientY - rect.top;
    
    const clicked = getAlbumAt(x, y);
    if (clicked) {
      onfocus(clicked);
    }
  }

  $effect(() => {
    ctrl.albums = albums;
  });

  $effect(() => {
    const _w = ctrl.layout.containerWidth;
    const _h = ctrl.viewportHeight;
    const _r = ctrl.virtualRows;
    const _p = prewarmer.pinnedTextures;
    
    if (_w > 0 && _h > 0) {
      wakeUp();
    }
  });

  let prevCols = 0;
  $effect(() => {
    const cols = ctrl.layout.cols;
    if (cols !== prevCols && prevCols !== 0) {
      const topAlbumIdx = ctrl.scroll.targetSlot * prevCols;
      const newSlot = Math.floor(topAlbumIdx / cols);
      ctrl.scroll.syncToSlot(newSlot);
      wakeUp();
    }
    prevCols = cols;
  });

  $effect(() => {
    const _v = version;
    ctrl.resetScroll();
    textCache.clear();
    wakeUp();
  });

  $effect(() => {
    const _c = config.album_grid;
    const _pal = config.palette;
    textCache.clear();
    emptyCoverCanvas = null;
    wakeUp();
  });

  onMount(() => {
    if (canvasEl) {
        ctx = canvasEl.getContext('2d', { alpha: false });
    }
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
    onpointermove={handlePointerMove}
    onpointerleave={handlePointerLeave}
    onclick={handleClick}
    aria-hidden="true"
  >
    <canvas 
      bind:this={canvasEl}
      style="display: block; width: 100%; height: 100%;"
    ></canvas>
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
</style>
