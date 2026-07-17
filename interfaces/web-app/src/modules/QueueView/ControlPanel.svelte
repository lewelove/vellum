<script lang="ts">
  import { player } from "../player.svelte.ts";
  import { collection } from "../../library/collection.svelte.ts";

  let currentFile = $derived(player.currentFile);
  let activeId = $derived(player.currentAlbumId);

  let fullAlbum = $derived(activeId ? collection.fullAlbumCache[activeId] : null);
  let currentTrackFull = $derived(fullAlbum?.tracks?.find((t: any) => `${fullAlbum.album.id}/${t.file?.path}` === currentFile) || null);

  let lyricsText = $state("");
  let isLoading = $state(false);
  let isInstrumental = $derived(currentTrackFull?.keys?.instrumental === true);

  let title = $derived(currentTrackFull?.title || player.title || "Unknown Title");
  let artist = $derived(currentTrackFull?.artist || player.artist || "Unknown Artist");

  let isPlaying = $derived(player.state === "play");
  let tickingElapsed = $state(0);
  let duration = $derived(player.duration || 0);
  let progress = $derived(duration > 0 ? (tickingElapsed / duration) * 100 : 0);

  function formatTime(totalSeconds: number) {
    const s = Math.floor(totalSeconds || 0);
    const m = Math.floor(s / 60);
    const rs = s % 60;
    const pad = (n: number) => String(n).padStart(2, '0');
    return `${m}:${pad(rs)}`;
  }

  $effect(() => {
    tickingElapsed = player.elapsed || 0;
  });

  $effect(() => {
    if (player.state !== "play") return;
    const startUpdated = player.lastUpdated;
    const startElapsed = player.elapsed;
    const currentDuration = player.duration || 0;
    const interval = setInterval(() => {
      const delta = (performance.now() - startUpdated) / 1000;
      tickingElapsed = Math.min(startElapsed + delta, currentDuration);
    }, 1000);
    return () => clearInterval(interval);
  });

  async function togglePlay() { try { await fetch('/api/toggle-pause', { method: 'POST' }); } catch(e) {} }
  async function next() { try { await fetch('/api/next', { method: 'POST' }); } catch(e) {} }
  async function prev() { try { await fetch('/api/prev', { method: 'POST' }); } catch(e) {} }

  async function fetchLyrics(trackFull: any) {
    if (!trackFull) {
      lyricsText = "";
      return;
    }

    if (trackFull.keys?.instrumental === true) {
      lyricsText = "";
      isLoading = false;
      return;
    }

    if (trackFull.lyrics && trackFull.lyrics.file) {
      isLoading = true;
      try {
          const encodedId = encodeURIComponent(activeId as string);
          const pathPart = trackFull.lyrics.file.path; 
          const url = `/api/assets/lyrics/${encodedId}/${pathPart}`;

          const res = await fetch(url);
          if (res.ok) {
              lyricsText = await res.text();
              isLoading = false;
              return;
          }
      } catch (e) {}
    }

    if (trackFull.keys && trackFull.keys.lyrics) {
      lyricsText = trackFull.keys.lyrics;
    } else {
      lyricsText = "";
    }

    isLoading = false;
  }

  $effect(() => {
    fetchLyrics(currentTrackFull);
  });
</script>

<div class="module-panel v-glass">
  <div class="panel-inner">
    <div class="control-panel-container">
      <div class="panel-header">
        <div class="header-content">
          <div class="header-row">
            <span class="v-truncate header-title">{title}</span>
          </div>
          <div class="header-row">
            <span class="v-truncate header-artist">{artist}</span>
          </div>
        </div>
      </div>

      <div class="lyrics-scroll">
        {#key currentFile}
          {#if isInstrumental}
            <div class="instrumental-msg">INSTRUMENTAL</div>
          {:else if lyricsText}
            <div class="lyrics-content">
              {#each lyricsText.split(/\r?\n/) as line}
                <p class="lyric-line">{line}</p>
              {/each}
            </div>
          {/if}
        {/key}
      </div>

      <div class="panel-footer">
        <div class="progress-container">
          <span class="time-display v-mono">{formatTime(tickingElapsed)}</span>
          <div class="progress-track">
            <div class="progress-fill" style="width: {progress}%"></div>
          </div>
          <span class="time-display v-mono">{formatTime(duration)}</span>
        </div>

        <div class="controls-container">
          <button class="v-btn-icon control-btn-lesser" onclick={prev} title="Previous">
            <img src="/icons/outlined/24px/skip_previous.svg" alt="" />
          </button>
          <button class="v-btn-icon control-btn" onclick={togglePlay} title="Toggle Play">
            <img src={isPlaying ? "/icons/outlined/24px/pause.svg" : "/icons/outlined/24px/play_arrow.svg"} alt="" />
          </button>
          <button class="v-btn-icon control-btn-lesser" onclick={next} title="Next">
            <img src="/icons/outlined/24px/skip_next.svg" alt="" />
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .module-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    clip-path: inset(0 -30px 0 0);
    box-shadow: none;
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

  .control-panel-container {
    width: 100%;
    height: 100%;
    background-color: transparent;
    box-sizing: border-box;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .panel-header {
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

  .lyrics-scroll {
    flex: 1;
    overflow-y: auto;
    width: 100%;
    min-height: 0;
    display: flex;
    flex-direction: column;
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

  .status-msg, .instrumental-msg {
    margin: auto;
    font-family: var(--font-mono);
    font-size: 14px;
    font-style: italic;
    display: flex;
    justify-content: center;
    align-items: center;
    height: 100%;
  }

  .status-msg {
    color: oklch(100% 0 0 / 0.8);
  }

  .instrumental-msg {
    font-size: 15px;
    line-height: 1.2;
    color: oklch(100% 0 0);
    text-align: center;
  }

  .panel-footer {
    flex: 0 0 auto;
    display: flex;
    flex-direction: column;
    padding-top: 16px;
    margin-top: 16px;
    border-top: 1px solid oklch(100% 0 0 / 0.07);
    gap: 16px;
  }

  .progress-container {
    display: flex;
    align-items: center;
    gap: 12px;
    width: 100%;
  }

  .time-display {
    font-size: 12px;
    color: oklch(100% 0 0 / 0.6);
    user-select: none;
    width: 40px;
    text-align: center;
  }

  .progress-track {
    flex: 1;
    height: 4px;
    background-color: oklch(100% 0 0 / 0.1);
    border-radius: 2px;
    position: relative;
    overflow: hidden;
  }

  .progress-fill {
    height: 100%;
    background-color: oklch(100% 0 0 / 0.5);
    border-radius: 2px;
  }

  .controls-container {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 8px;
  }

  .control-btn {
    width: 36px;
    height: 36px;
    border-radius: 18px;
    flex-shrink: 0;
  }

  .control-btn img {
    width: 20px;
    height: 20px;
  }

  .control-btn-lesser {
    width: 32px;
    height: 32px;
    border-radius: 16px;
    flex-shrink: 0;
  }

  .control-btn-lesser img {
    width: 18px;
    height: 18px;
  }
</style>
