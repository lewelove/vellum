<script lang="ts">
  import { onMount, onDestroy } from "svelte";
  import { player } from "../player.svelte.ts";
  import { library } from "../../library.svelte.ts";
  import { nav } from "../../navigation.svelte.ts";
  import { fade } from "svelte/transition";
  
  import TracklistPanel from "./TracklistPanel.svelte";
  import Sidebar from "./Sidebar.svelte";
  import LyricsPanel from "./LyricsPanel.svelte";
  import ClearCover from "../ClearCover.svelte";
  import BackgroundShader from "./BackgroundShader.svelte";
  import NavBar from "../NavigationBar/NavBar.svelte";
  import CoverPanel from "./CoverPanel.svelte";

  let activeId = $derived(player.currentAlbumId);
  let activeAlbum = $derived(activeId ? library.dict[activeId] : null);
  let coverHash = $derived(activeAlbum?.cover_hash || "");
  
  let fullAlbum = $derived(activeId ? library.fullAlbumCache[activeId] : null);

  let palette = $derived(fullAlbum?.album?.tags?.cover_palette || activeAlbum?.tags?.cover_palette || []);
  let hasPalette = $derived(palette && palette.length > 0);
  let hasLyrics = $derived(fullAlbum?.tracks?.some((t: any) => !!t.info?.lyrics_path || t.tags?.instrumental === true) ?? false);

  let isViewVisible = $derived(nav.activeTab === 'queue');
  let isPlaying = $derived(player.state === "play");

  let showLyricsPanel = $derived(library.queuePanels.lyrics && hasLyrics);
  let showTracksPanel = $derived(library.queuePanels.tracks);

  let moduleWidth = $state(0);

  let isExpanded = $state(false);
  let windowWidth = $state(0);
  let windowHeight = $state(0);

  let expandedSize = $derived.by(() => {
    if (windowWidth <= 0 || windowHeight <= 0) return 0;
    return Math.min(windowWidth, windowHeight) - 48; 
  });

  let glassOpacity = $derived.by(() => {
    if (!palette || palette.length === 0) return 0.5;
    
    const lValues = palette.map((entry: any) => {
      if (!Array.isArray(entry) || entry.length < 2) return 0.5;
      const match = entry[1].match(/oklch\(([\d.]+)%/);
      return match ? parseFloat(match[1]) / 100 : 0.5;
    });

    const maxL = Math.max(...lValues);
    return (Math.abs(maxL - 0.5) * 0.6 + 0.2).toFixed(3);
  });

  $effect(() => {
    if (activeId) {
      library.ensureFullAlbum(activeId);
    }
  });

  function toggleExpand() {
    if (coverHash) {
      isExpanded = !isExpanded;
    }
  }

  function handleKeydown(e: KeyboardEvent) {
    if (isExpanded && e.key === "Escape") {
      isExpanded = false;
    }
  }

  onMount(() => {
    window.addEventListener("keydown", handleKeydown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", handleKeydown);
  });
</script>

<svelte:window bind:innerWidth={windowWidth} bind:innerHeight={windowHeight} />

<div 
  class="queue-view-container" 
  class:shader-off={!library.isShaderActive || !hasPalette}
  style="--glass-bg: oklch(26% 0 0 / {glassOpacity});"
>
  <BackgroundShader colors={palette} coverSize={moduleWidth} visible={isViewVisible} {isPlaying} />

  <NavBar variant={library.isShaderActive && hasPalette ? 'glass' : 'solid'} />

  {#if isExpanded}
    <div 
      class="expanded-backdrop" 
      onclick={toggleExpand}
      transition:fade={{ duration: 200 }}
    >
      <div 
        class="expanded-content" 
        style="width: {expandedSize}px; height: {expandedSize}px;"
        onclick={(e) => e.stopPropagation()} 
        role="presentation"
      >
        <div in:fade={{ duration: 200 }}>
          <ClearCover 
            hash={coverHash} 
            width={expandedSize} 
            height={expandedSize} 
          />
        </div>
      </div>
    </div>
  {/if}
  
  <div class="view-content-wrapper">
    <div class="queue-modules">
      
      <div class="side-column lyrics-col">
        {#if showLyricsPanel}
          <div class="module-panel v-glass">
            <div class="panel-inner">
              <LyricsPanel />
            </div>
          </div>
        {/if}
      </div>

      <div class="center-column">
        <CoverPanel 
          {coverHash} 
          bind:width={moduleWidth} 
          onclick={toggleExpand} 
        />
      </div>

      <div class="side-column tracks-col">
        {#if showTracksPanel}
          <div class="module-panel v-glass">
            <div class="panel-inner">
              <TracklistPanel />
            </div>
          </div>
        {/if}
      </div>

    </div>
  </div>

  <Sidebar {hasLyrics} {hasPalette} />
</div>

<style>
  .queue-view-container {
    width: 100%;
    height: 100%;
    background-color: transparent;
    position: relative;
    overflow: hidden;
    display: flex;
    flex-direction: row;
  }

  .view-content-wrapper {
    flex: 1;
    position: relative;
    height: 100%;
    min-width: 0;
    padding: 0;
    box-sizing: border-box;
    z-index: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .queue-modules {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: row;
    align-items: stretch;
  }

  .side-column {
    flex: 1;
    display: flex;
    flex-direction: column;
    height: 100%;
    min-width: 0;
    overflow: visible;
  }

  .lyrics-col .module-panel {
    clip-path: inset(0 -30px 0 0);
  }

  .tracks-col .module-panel {
    clip-path: inset(0 0 0 -30px);
  }

  .center-column {
    flex: 0 0 auto;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    min-width: 0;
    padding: 24px;
    box-sizing: border-box;
  }

  .module-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-inner {
    flex: 1;
    padding: 20px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    text-shadow: 0 1px 3px oklch(0% 0 0 / 0.3);
    min-height: 0;
  }

  .expanded-backdrop {
    position: fixed;
    inset: 0;
    z-index: 9999;
    background-color: rgba(0, 0, 0, 0.2);
    backdrop-filter: blur(16px);
    display: flex;
    align-items: center;
    justify-content: center;
    cursor: pointer;
  }

  .expanded-content {
    position: relative;
    z-index: 10000;
    pointer-events: none;
  }
</style>
