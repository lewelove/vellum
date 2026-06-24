<script lang="ts">
  import { player } from "../player.svelte.ts";
  import { library } from "../../library.svelte.ts";
  import { jumpToQueueIndex } from "../../api.ts";
  import { setTab } from "../../navigation.svelte.ts";

  let { showHud, hasPalette } = $props();

  let activeId = $derived(player.currentAlbumId);
  let isStopped = $derived(player.state === "stop");

  async function handleFocus() {
    if (activeId) {
      await setTab("home");
      await library.setFocus({ id: activeId });
    }
  }

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

  function formatMs(ms: number) {
    if (!ms) return "0:00";
    const totalSeconds = Math.floor(ms / 1000);
    const h = Math.floor(totalSeconds / 3600);
    const m = Math.floor((totalSeconds % 3600) / 60);
    const s = totalSeconds % 60;
    const pad = (num: number) => String(num).padStart(2, '0');
    if (h > 0) return `${h}:${pad(m)}:${pad(s)}`;
    return `${m}:${pad(s)}`;
  }

  function getDiscDuration(tracks: any[], discNumber: number) {
    const totalMs = tracks
      .filter(t => t.discNo === discNumber)
      .reduce((acc, t) => acc + t.durationMs, 0);
    return formatMs(totalMs);
  }

  async function handleJump(id: string | number) {
    try { await jumpToQueueIndex(id); } catch (e) {}
  }

  let mappedTracks = $derived(player.queue.map(item => {
    const fullAlbum = item.album_id ? library.fullAlbumCache[item.album_id] : null;
    const meta = fullAlbum?.tracks?.find((t: any) => `${fullAlbum.album.id}/${t.file?.path}` === item.file);

    return {
      id: item.id,
      file: item.file,
      isPlaying: player.currentFile === item.file,
      trackNo: meta ? meta.tracknumber : "#",
      discNo: meta ? meta.discnumber : 1,
      duration: meta ? meta.info?.duration_formatted : "",
      durationMs: meta ? meta.info?.duration_milliseconds : 0,
      title: meta ? meta.title : (item.title || item.file),
      artist: meta ? meta.artist : (item.artist || ""),
      albumId: item.album_id || null
    };
  }));

  let groupedQueue = $derived.by(() => {
    const groups: any[] = [];
    mappedTracks.forEach(track => {
      if (groups.length === 0 || groups[groups.length - 1].albumId !== track.albumId) {
        const albumMeta = library.dict[track.albumId];
        groups.push({
          albumId: track.albumId,
          albumMeta,
          tracks: [track]
        });
      } else {
        groups[groups.length - 1].tracks.push(track);
      }
    });
    return groups;
  });
</script>

{#snippet NavButton({ icon, label, disabled, active, onclick }: { icon: string, label: string, disabled?: boolean, active?: boolean, onclick: () => void })}
  <button class="v-btn-icon queue-nav-button" class:active {disabled} {onclick} title={label}>
    <img src="/{icon}" alt={label} class="nav-icon" />
  </button>
{/snippet}

{#snippet NavButtons()}
  {@render NavButton({ icon: "icons/outlined/24px/side_navigation.svg", label: "Toggle HUD", active: showHud, onclick: () => library.toggleQueuePanel('hud') })}
  {#if hasPalette}
    {@render NavButton({ 
      icon: "icons/outlined/24px/colors.svg", 
      label: "Toggle Shader", 
      active: library.isShaderEnabled, 
      disabled: isStopped,
      onclick: () => library.toggleShader() 
    })}
  {/if}
  {@render NavButton({
    icon: "icons/outlined/24px/album.svg",
    label: "Focus Album",
    disabled: !activeId,
    onclick: handleFocus
  })}
{/snippet}

{#if showHud}
  <div class="module-panel v-glass">
    <div class="panel-inner">
      <div class="tracks-list-container">
        <div class="tracks-list">
          {#each groupedQueue as group}
            {#if group.albumMeta}
              <div class="album-group-header">
                <div class="header-content">
                  <div class="header-row">
                    <span class="v-truncate header-album">{group.albumMeta.album}</span>
                    <span class="v-mono header-meta">{group.albumMeta.date?.substring(0,4)}</span>
                  </div>
                  <div class="header-row">
                    <span class="v-truncate header-artist">{group.albumMeta.albumartist}</span>
                    <span class="v-mono header-meta">{group.albumMeta.album_duration_time}</span>
                  </div>
                </div>
              </div>
            {/if}

            {@const isMultiDiscAlbum = group.albumMeta && parseInt(group.albumMeta.total_discs || "1") > 1}

            {#each group.tracks as track, i (track.id)}
              {@const showDiscHeader = isMultiDiscAlbum && (i === 0 || track.discNo !== group.tracks[i-1].discNo)}

              {#if showDiscHeader}
                {#if i > 0}
                  <div class="disc-separator"></div>
                {/if}
                <div class="disc-header-row" class:first-disc={i === 0}>
                  <span class="disc-label">Disc {track.discNo}</span>
                  <div class="disc-header-right">
                    <span class="v-mono disc-duration-label">{getDiscDuration(group.tracks, track.discNo)}</span>
                  </div>
                </div>
              {/if}

              <div 
                class="v-track-row track-row" 
                class:active={track.isPlaying}
                ondblclick={() => handleJump(track.id)}
                onkeydown={(e) => { if (e.key === 'Enter') { e.preventDefault(); handleJump(track.id); } }}
                role="button"
                tabindex="0"
              >
                <div class="v-track-body track-body">
                  <span class="v-truncate v-track-title track-title">{track.title}</span>
                  {#if track.artist && group.albumMeta && track.artist.toLowerCase() !== group.albumMeta.albumartist.toLowerCase()}
                    <span class="v-truncate v-track-artist track-artist">{track.artist}</span>
                  {/if}
                </div>
                <span class="v-mono v-track-meta track-meta">
                  {formatDuration(track.duration)}
                </span>
              </div>
            {/each}
          {/each}
        </div>
      </div>
    </div>

    <div class="panel-splitter"></div>

    <div class="sidebar-buttons">
      {@render NavButtons()}
    </div>
  </div>
{:else}
  <div class="floating-buttons">
    {@render NavButtons()}
  </div>
{/if}

<style>
  .module-panel {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
    clip-path: inset(0 0 0 -30px);
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

  .panel-splitter {
    height: 1px;
    background-color: oklch(100% 0 0 / 0.07);
    margin: 0 20px;
    flex-shrink: 0;
  }

  .sidebar-buttons {
    display: flex;
    flex-direction: row;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    padding: 16px 20px;
    flex-shrink: 0;
  }

  .floating-buttons {
    display: flex;
    flex-direction: row;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    padding: 16px 20px;
    margin-top: auto;
  }

  .queue-nav-button {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    box-shadow: var(--button-shadow-lesser);
    flex-shrink: 0;
    pointer-events: auto;
  }

  .queue-nav-button:disabled {
    opacity: 0.3;
    pointer-events: none;
    box-shadow: none;
  }

  .nav-icon {
    width: 20px;
    height: 20px;
  }

  .header-album,
  .track-title {
    color: oklch(100% 0 0);
  }

  .header-meta {
    color: oklch(100% 0 0 / 0.6);
  }

  .header-artist,
  .track-artist,
  .disc-label,
  .disc-duration-label,
  .track-meta {
    color: oklch(100% 0 0 / 0.7);
  }

  .disc-label,
  .disc-duration-label {
    color: oklch(100% 0 0 / 0.6);
  }

  .tracks-list-container {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    box-sizing: border-box;
    background-color: transparent;
    min-height: 0;
    overflow: hidden;
  }

  .tracks-list {
    flex: 1;
    overflow-y: auto;
    min-height: 0;
  }

  .tracks-list::-webkit-scrollbar {
    width: 0px;
  }

  .album-group-header {
    padding: 0px 0px 16px 0px;
    display: flex;
    align-items: center;
    box-sizing: border-box;
    border-bottom: 1px solid oklch(100% 0 0 / 0.07);
    margin-bottom: 16px;
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
  }

  .header-album {
    font-size: 19px;
  }

  .header-artist {
    font-size: 17px;
  }

  .header-meta {
    font-size: 15px;
    margin-left: 8px;
  }

  .disc-separator {
    height: 1px;
    background-color: oklch(100% 0 0 / 0.07);
    margin: 10px 0 12px 0;
  }

  .disc-header-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    padding: 0 0px;
    margin: 12px 0px 8px 0;
  }

  .disc-header-row.first-disc {
    margin-top: 0px;
  }

  .disc-header-right {
    display: flex;
    align-items: center;
  }

  .disc-label,
  .disc-duration-label {
    display: flex;
    align-items: center;
    padding: 0 12px;
    font-size: 12px;
    border: 1px solid oklch(100% 0 0 / 0.07);
    border-radius: 8px;
    height: 24px;
    box-sizing: border-box;
  }

  .disc-label {
    font-weight: 500;
  }

  .disc-duration-label {
    font-weight: 400;
  }

  .track-row {
    margin: 0 0px;
    padding-left: 14px;
    transition: none !important;
  }

  .track-row + .track-row {
    margin-top: 4px;
  }
</style>
