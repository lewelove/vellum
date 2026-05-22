<script lang="ts">
  import { fade } from "svelte/transition";
  import { cubicOut } from "svelte/easing";
  import { 
    playAlbum, 
    queueAlbum, 
    openAlbumFolder, 
    playDisc,
    openLockFile,
    openManifestFile,
    updateAlbum 
  } from "../../../api.ts";
  import { library } from "../../../library.svelte.ts";
  import ClearCover from "../../ClearCover.svelte";
  import ModalDrawerTracks from "./ModalDrawerTracks.svelte";

  let { album, onclose }: { album: any, onclose: () => void } = $props();

  let windowWidth = $state(typeof window !== "undefined" ? window.innerWidth : 1280);
  let leftColumnWidth = $derived(Math.round(0.36 * windowWidth));

  let albumData = $derived(album.album || {});
  let infoData = $derived(albumData.info || {});

  let coverHash = $derived(infoData.cover_hash || "");
  let title = $derived(albumData.album || "Untitled");
  let artist = $derived(albumData.albumartist || "Unknown");
  let genreString = $derived(Array.isArray(albumData.genre) ? albumData.genre.join(" ; ") : (albumData.genre || ""));
  let dateString = $derived(albumData.date || "");

  let discCount = $derived(parseInt(infoData.total_discs || "1"));
  let trackCount = $derived(parseInt(infoData.total_tracks || "0"));
  let durationStr = $derived(infoData.album_duration_time || "--:--");

  let coverIsReady = $state(false);

  $effect(() => {
    if (!coverHash) {
      coverIsReady = true;
      return;
    }
    const dpr = window.devicePixelRatio || 1;
    const imgWidth = leftColumnWidth - 64;
    const targetWidth = Math.round(imgWidth * dpr);
    const srcUrl = `/api/resize/${targetWidth}px/${coverHash}?v=${coverHash}`;
    
    const img = new Image();
    img.src = srcUrl;
    img.onload = () => {
      coverIsReady = true;
    };
    img.onerror = () => {
      coverIsReady = true;
    };
  });

  async function handlePlay() {
    try { await playAlbum(album.id); } catch (err) { console.error(err); }
  }

  async function handleQueue() {
    try { await queueAlbum(album.id); } catch (err) { console.error(err); }
  }

  async function handleOpenFolder() {
    try { await openAlbumFolder(album.id); } catch (err) { console.error(err); }
  }

  async function handleOpenLock() {
    try { await openLockFile(album.id); } catch (err) { console.error(err); }
  }

  async function handleOpenManifest() {
    try { await openManifestFile(album.id); } catch (err) { console.error(err); }
  }

  async function handleUpdate() {
    try { await updateAlbum(album.id); } catch (err) { console.error(err); }
  }

  async function handlePlayTrack(index: number) {
    try {
      const track = album.tracks[index];
      const discNumber = track.discnumber;
      
      let intraDiscOffset = 0;
      for (let i = 0; i < index; i++) {
        if (album.tracks[i].discnumber === discNumber) {
          intraDiscOffset++;
        }
      }
      
      await playDisc(album.id, discNumber, intraDiscOffset);
    } catch (err) { 
      console.error(err); 
    }
  }

  async function handlePlayDisc(discNumber: number) {
    try { await playDisc(album.id, discNumber); } catch (err) { console.error(err); }
  }

  function handleBackdropClick(e: MouseEvent) {
    if (e.target === e.currentTarget) {
      onclose();
    }
  }
</script>

<svelte:window bind:innerWidth={windowWidth} />

{#if coverIsReady}
  <div 
    class="modal-backdrop" 
    onclick={handleBackdropClick} 
    role="presentation"
    transition:fade={{ duration: 200, easing: cubicOut }}
  >
    <div class="modal-chassis v-panel">
      <div class="modal-content">
        
        <div class="column-left">
          <div class="cover-container" style="height: {leftColumnWidth - 64}px;">
            <ClearCover 
              hash={coverHash} 
              width={leftColumnWidth - 64} 
              height={leftColumnWidth - 64} 
              animate={false}
            />
          </div>

          <div class="meta-container">
            <h2 class="album-title">{title}</h2>
            <h3 class="album-artist">{artist}</h3>

            {#if dateString}
              <span class="v-mono meta-date">{dateString}</span>
            {/if}
            
            <div class="meta-stack">
              <div class="v-mono meta-row">
                <span class="v-truncate meta-val">{durationStr}</span>
                
                {#if discCount > 1}
                  <span class="meta-sep">•</span>
                  <span class="v-truncate meta-val">{discCount} Discs</span>
                {/if}

                <span class="meta-sep">•</span>
                <span class="v-truncate meta-val">{trackCount} Tracks</span>
              </div>
            </div>
          </div>
        </div>

        <div class="column-right">
          <div class="button-bar">
            <div class="bar-group">
              <button class="v-btn-icon icon-btn" onclick={handleUpdate} title="Update Album">
                <img src="/icons/outlined/24px/refresh.svg" alt="Update"/>
              </button>
              <button class="v-btn-icon icon-btn" onclick={handleOpenFolder} title="Open Local Folder">
                <img src="/icons/outlined/24px/folder.svg" alt="Open"/>
              </button>
              <button class="v-btn-icon icon-btn" onclick={handleOpenManifest} title="Open Manifest">
                <img src="/icons/outlined/24px/edit_document.svg" alt="Manifest"/>
              </button>
              <button class="v-btn-icon icon-btn" onclick={handleOpenLock} title="Open Data Object">
                <img src="/icons/outlined/24px/code.svg" alt="Data Object"/>
              </button>
            </div>

            <div class="bar-group right">
              <button class="v-btn-icon icon-btn" onclick={handlePlay} title="Play Album">
                <img src="/icons/outlined/24px/play_arrow.svg" alt="" />
              </button>
            </div>
          </div>
          <div class="tracks-scroll-area">
            <div class="v-scroll-fade-top"></div>
            <ModalDrawerTracks 
              tracks={album.tracks ||[]} 
              totalDiscs={infoData.total_discs} 
              albumArtist={artist}
              onplay={handlePlayTrack} 
              onplaydisc={handlePlayDisc}
            />
            <div class="v-scroll-fade-bottom"></div>
          </div>
        </div>

      </div>
    </div>
  </div>
{/if}

<style>
  .button-bar {
    display: flex;
    justify-content: flex-end;
    align-items: center;
    gap: 10px;
    padding-bottom: 12px;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
    width: 100%;
  }

  .bar-group {
    display: flex;
    align-items: center;
    gap: 10px;
  }

  .bar-group.right {
    margin-left: auto;
  }

  .modal-backdrop {
    position: fixed;
    inset: 0;
    background-color: oklch(0% 0 0 / 0.5);
    backdrop-filter: blur(2px);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 1000;
  }

  .modal-chassis {
    width: 80vw;
    height: 85vh;
  }

  .modal-content {
    display: grid;
    grid-template-columns: 45% 55%;
    grid-template-rows: 100%;
    height: 100%;
    width: 100%;
    min-height: 0;
  }

  .column-left {
    display: flex;
    flex-direction: column;
    padding: 32px;
    background-color: #1f1f1f;
    min-width: 0;
    min-height: 0;
    box-sizing: border-box;
  }

  .cover-container {
    width: 100%;
    flex-shrink: 0;
    background-color: transparent;
    overflow: visible;
  }

  .meta-container {
    margin-top: 16px;
    display: flex;
    flex-direction: column;
    flex: 1;
    min-width: 0;
  }

  .album-title {
    margin: 0;
    font-size: 25px;
    font-weight: 400;
    color: var(--text-main);
    word-wrap: break-word;
  }

  .album-artist {
    margin: 5px 0 0 0;
    font-size: 20px;
    font-weight: 400;
    color: var(--text-muted);
    word-wrap: break-word;
  }

  .meta-stack {
    margin-top: auto;
    display: flex;
    flex-direction: column;
    gap: 8px;
  }

  .meta-row, .meta-date {
    display: flex;
    align-items: center;
    font-size: 16px;
    color: #888888;
    gap: 12px;
  }

  .meta-date {
    margin: 12px 0 0 0;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .meta-sep {
    color: #777777;
    font-size: 12px;
    flex-shrink: 0;
  }

  .column-right {
    display: flex;
    flex-direction: column;
    padding: 32px;
    min-width: 0;
    min-height: 0;
    height: 100%;
    box-sizing: border-box;
    background-color: var(--background-drawer);
  }

  .icon-btn {
    width: 36px;
    height: 36px;
    border-radius: 18px;
  }

  .icon-btn img {
    width: 18px;
    height: 18px;
  }

  .tracks-scroll-area {
    position: relative;
    flex: 1;
    overflow-y: scroll;
    min-height: 0;
    background-color: var(--background-drawer);
    transform: translateZ(0);
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .tracks-scroll-area::-webkit-scrollbar {
    width: 0px;
  }
</style>
