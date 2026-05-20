<script lang="ts">
  import { player } from "../player.svelte.ts";
  import { library } from "../../library.svelte.ts";

  let currentFile = $derived(player.currentFile);
  let activeId = $derived(player.currentAlbumId);
  
  let fullAlbum = $derived(activeId ? library.fullAlbumCache[activeId] : null);
  let currentTrackFull = $derived(fullAlbum?.tracks?.find((t: any) => t.info?.track_library_path === currentFile) || null);

  let lyricsText = $state("");
  let isLoading = $state(false);
  
  let isInstrumental = $derived(currentTrackFull?.tags?.instrumental === true);

  let title = $derived(currentTrackFull?.title || player.title || "Unknown Title");
  let artist = $derived(currentTrackFull?.artist || player.artist || "Unknown Artist");
  let durationStr = $derived(currentTrackFull?.info?.track_duration_time ? formatDuration(currentTrackFull.info.track_duration_time) : "");
  let trackNoStr = $derived(currentTrackFull?.tracknumber ? `Track ${currentTrackFull.tracknumber}` : "");

  function formatDuration(str: string) {
    if (!str) return "0:00";
    let parts = str.split(':');
    while (parts.length > 2 && parseInt(parts[0]) === 0) {
      parts.shift();
    }
    if (parts[0].length > 1 && parts[0].startsWith('0')) {
      parts[0] = parts[0].substring(1);
    }
    return parts.join(':');
  }

  async function fetchLyrics(trackFull: any) {
    if (!trackFull) {
      lyricsText = "";
      return;
    }

    if (trackFull.tags?.instrumental === true) {
      lyricsText = "";
      isLoading = false;
      return;
    }

    if (trackFull.info?.lyrics_path) {
      isLoading = true;
      try {
          const encodedId = encodeURIComponent(activeId as string);
          const pathPart = trackFull.info.lyrics_path; 
          const url = `/api/assets/lyrics/${encodedId}/${pathPart}`;
          
          const res = await fetch(url);
          if (res.ok) {
              lyricsText = await res.text();
              isLoading = false;
              return;
          }
      } catch (e) {}
    }
    
    if (trackFull.tags && trackFull.tags.lyrics) {
      lyricsText = trackFull.tags.lyrics;
    } else {
      lyricsText = "";
    }
    
    isLoading = false;
  }

  $effect(() => {
    fetchLyrics(currentTrackFull);
  });
</script>

<div class="lyrics-container">
  <div class="lyrics-header">
    <div class="header-content">
      <div class="header-row">
        <span class="v-truncate header-title">{title}</span>
      </div>
      <div class="header-row">
        <span class="v-truncate header-artist">{artist}</span>
      </div>
    </div>
  </div>

  {#key currentFile}
    <div class="lyrics-scroll">
      {#if isLoading}
        <div class="status-msg">Loading...</div>
      {:else if isInstrumental}
        <div class="instrumental-msg">INSTRUMENTAL</div>
      {:else if lyricsText}
        <div class="lyrics-content">
          {#each lyricsText.split(/\r?\n/) as line}
            <p class="lyric-line">{line}</p>
          {/each}
        </div>
      {:else}
        <div class="status-msg">
          No lyrics available.
        </div>
      {/if}
    </div>
  {/key}
</div>

<style>
  .lyrics-container {
    width: 100%;
    height: 100%;
    background-color: transparent;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .lyrics-header {
    flex: 0 0 auto;
    padding: 0px 0px 16px 0px;
    display: flex;
    align-items: center;
    box-sizing: border-box;
    border-bottom: 1px solid oklch(100% 0 0 / 0.07);
    margin-bottom: 16px;
    width: 100%;
  }

  .header-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    justify-content: center;
    min-width: 0;
    gap: 6px;
  }

  .header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
  }

  .header-title {
    color: oklch(100% 0 0);
    font-size: 19px;
  }

  .header-artist {
    color: oklch(100% 0 0 / 0.7);
    font-size: 17px;
  }

  .header-meta {
    color: oklch(100% 0 0 / 0.6);
    font-size: 15px;
    margin-left: 8px;
  }

  .lyrics-scroll {
    flex: 1;
    overflow-y: auto;
    width: 100%;
    min-height: 0;
  }

  .lyrics-scroll::-webkit-scrollbar {
    width: 0px;
  }

  .lyrics-content {
    font-family: var(--font-stack);
    font-size: 15px;
    line-height: 1.05;
    color: var(--text-main);
    text-align: center;
    margin: -6px auto;
    width: 100%;
  }

  .lyric-line {
    margin: 6px 0;
    min-height: 0.3em;
    text-wrap: balance;
  }

  .status-msg {
    margin: auto;
    color: oklch(100% 0 0 / 0.8);
    font-family: var(--font-mono);
    font-size: 14px;
    font-style: italic;
  }

  .instrumental-msg {
    font-family: var(--font-mono);
    font-size: 15px;
    font-style: italic;
    line-height: 1.2;
    color: oklch(100% 0 0);
    margin: auto;
    text-align: center;
  }
</style>
