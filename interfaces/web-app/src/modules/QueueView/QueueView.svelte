<script lang="ts">
  import { player } from "../player.svelte.ts";
  import { collection } from "../../library/collection.svelte.ts";
  import { view } from "../../library/view.svelte.ts";
  import { nav } from "../../navigation.svelte.ts";
  
  import TracklistPanel from "./TracklistPanel.svelte";
  import ControlPanel from "./ControlPanel.svelte";
  import BackgroundShader from "./BackgroundShader.svelte";
  import NavBar from "../NavigationBar/NavBar.svelte";
  import CoverPanel from "./CoverPanel.svelte";

  let activeId = $derived(player.currentAlbumId);
  let activeAlbum = $derived(activeId ? collection.dict[activeId] : null);
  let coverHash = $derived(activeAlbum?.cover_hash || "");
  let fullAlbum = $derived(activeId ? collection.fullAlbumCache[activeId] : null);

  let palette = $derived(fullAlbum?.album?.keys?.cover_palette || activeAlbum?.keys?.cover_palette || []);
  let hasPalette = $derived(palette && palette.length > 0);

  let isViewVisible = $derived(nav.activeTab === 'queue');
  let isPlaying = $derived(player.state === "play");

  let moduleWidth = $state(0);

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
    const uniqueIds = [...new Set(player.queue.map(item => item.album_id).filter(Boolean))];
    if (activeId && !uniqueIds.includes(activeId)) {
      uniqueIds.push(activeId);
    }
    uniqueIds.forEach(id => collection.ensureFullAlbum(id));
  });
</script>

<div 
  class="queue-view-container" 
  class:shader-off={!view.isShaderActive || !hasPalette}
  style="--glass-bg: oklch(26% 0 0 / {glassOpacity});"
>
  <BackgroundShader colors={palette} coverSize={moduleWidth} visible={isViewVisible} {isPlaying} />

  <div class="queue-layout">
    <div class="left-wing">
      <NavBar variant="transparent" />
      <ControlPanel />
    </div>

    <div class="center-wing">
      <CoverPanel {coverHash} bind:width={moduleWidth} />
    </div>

    <div class="right-wing">
      <TracklistPanel {hasPalette} />
    </div>
  </div>
</div>

<style>
  .queue-view-container {
    width: 100%;
    height: 100%;
    background-color: transparent;
    position: relative;
    overflow: hidden;
  }

  .queue-layout {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: row;
    z-index: 1;
    position: relative;
  }

  .left-wing, .right-wing {
    flex: 1 1 0%;
    display: flex;
    min-width: 0;
    height: 100%;
  }

  .left-wing {
    flex-direction: row;
  }

  .right-wing {
    flex-direction: column;
  }

  .center-wing {
    flex: 0 0 auto;
    height: 100%;
    display: flex;
    justify-content: center;
    align-items: center;
    box-sizing: border-box;
    min-width: 0;
  }
</style>
