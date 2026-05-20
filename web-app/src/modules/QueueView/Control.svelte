<script lang="ts">
  import { player } from "../player.svelte.ts";

  let isPlaying = $derived(player.state === "play");

  async function togglePlay() { 
    try { await fetch('/api/toggle-pause', { method: 'POST' }); } catch(e) {} 
  }
  
  async function next() { 
    try { await fetch('/api/next', { method: 'POST' }); } catch(e) {} 
  }
  
  async function prev() { 
    try { await fetch('/api/prev', { method: 'POST' }); } catch(e) {} 
  }

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
</script>

<div class="control-container">
  <div class="buttons">
    <button class="v-btn-icon control-btn-lesser" onclick={prev} title="Previous">
      <img src="/icons/outlined/24px/skip_previous.svg" alt="" class="rotated-icon" />
    </button>
    <button class="v-btn-icon control-btn" onclick={togglePlay} title="Toggle Play">
      <img src={isPlaying ? "/icons/outlined/24px/pause.svg" : "/icons/outlined/24px/play_arrow.svg"} alt="" />
    </button>
    <button class="v-btn-icon control-btn-lesser" onclick={next} title="Next">
      <img src="/icons/outlined/24px/skip_next.svg" alt="" class="rotated-icon" />
    </button>
  </div>

  <div class="time-display top v-mono">
    {formatTime(tickingElapsed)}
  </div>

  <div class="progress-track">
    <div class="progress-fill" style="height: {progress}%"></div>
  </div>

  <div class="time-display bottom v-mono">
    {formatTime(duration)}
  </div>
</div>

<style>
  .control-container {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    height: 100%;
    width: 100%;
  }

  .time-display {
    font-size: 10px;
    color: oklch(100% 0 0 / 0.6);
    text-align: center;
    width: 100%;
    user-select: none;
  }

  .time-display.top {
    margin-top: 8px;
    margin-bottom: 8px;
  }

  .time-display.bottom {
    margin-top: 8px;
    margin-bottom: 8px;
  }

  .progress-track {
    width: 4px;
    flex: 1;
    position: relative;
    min-height: 60px;
    background-color: oklch(100% 0 0 / 0.1);
    border-radius: 2px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    justify-content: flex-start;
  }

  .progress-fill {
    width: 100%;
    background-color: oklch(100% 0 0 / 0.5);
    border-radius: 2px;
  }

  .buttons {
    display: flex;
    flex-direction: column;
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
    margin: 0 2px;
    border-radius: 20px;
    flex-shrink: 0;
  }

  .control-btn-lesser img {
    width: 18px;
    height: 18px;
  }

  .rotated-icon {
    transform: rotate(90deg);
  }
</style>
