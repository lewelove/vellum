<script lang="ts">
  import { fade } from "svelte/transition";

  let { items = [], container = null }: { items?: any[], container?: HTMLElement | null } = $props();

  const ALL_CHARS = "ABCDEFGHIJKLMNOPQRSTUVWXYZ#".split("");
  
  let indexContainer: HTMLDivElement | null = $state(null);
  let charsWrapper: HTMLDivElement | null = $state(null);
  let isScrubbing = $state(false);
  let scrubChar = $state("");
  let bubbleY = $state(0);

  function getBucketChar(label: string) {
    if (!label) return "#";
    const normalized = label.normalize("NFD").replace(/[\u0300-\u036f]/g, "").toUpperCase();
    const char = normalized.charAt(0);
    return /[A-Z]/.test(char) ? char : "#";
  }

  let charJumpMap = $derived.by(() => {
    const map = new Map();
    items.forEach((item, i) => {
      const char = getBucketChar(item.label);
      if (!map.has(char)) {
        map.set(char, i);
      }
    });

    let nextAvailableIdx = items.length > 0 ? items.length - 1 : 0;
    for (let i = ALL_CHARS.length - 1; i >= 0; i--) {
      const char = ALL_CHARS[i];
      if (map.has(char)) {
        nextAvailableIdx = map.get(char);
      } else {
        map.set(char, nextAvailableIdx);
      }
    }
    return map;
  });

  function calculateScrub(e: PointerEvent) {
    if (!charsWrapper || !indexContainer || ALL_CHARS.length === 0) return;
    
    const wrapperRect = charsWrapper.getBoundingClientRect();
    const containerRect = indexContainer.getBoundingClientRect();
    
    const wrapperY = e.clientY - wrapperRect.top;
    bubbleY = Math.max(0, Math.min(e.clientY - containerRect.top, containerRect.height));

    let pct = wrapperY / wrapperRect.height;
    pct = Math.max(0, Math.min(1, pct));
    
    const charIndex = Math.min(Math.floor(pct * ALL_CHARS.length), ALL_CHARS.length - 1);
    const targetChar = ALL_CHARS[charIndex];
    
    if (targetChar !== scrubChar) {
      scrubChar = targetChar;
      
      if (container) {
        const jumpIndex = charJumpMap.get(targetChar);
        if (jumpIndex !== undefined && jumpIndex < items.length) {
          const targetEl = container.querySelector(`#sidebar-item-${jumpIndex}`) as HTMLElement;
          if (targetEl) {
            container.scrollTop = targetEl.offsetTop - 12;
          }
        } else if (jumpIndex === items.length - 1 && items.length > 0) {
          const targetEl = container.querySelector(`#sidebar-item-${jumpIndex}`) as HTMLElement;
          if (targetEl) {
             container.scrollTop = targetEl.offsetTop;
          }
        }
      }
    }
  }

  function onPointerDown(e: PointerEvent) {
    if (e.button !== 0) return;
    isScrubbing = true;
    indexContainer?.setPointerCapture(e.pointerId);
    calculateScrub(e);
  }

  function onPointerMove(e: PointerEvent) {
    if (isScrubbing) calculateScrub(e);
  }

  function onPointerUp(e: PointerEvent) {
    isScrubbing = false;
    scrubChar = "";
    if (indexContainer && indexContainer.hasPointerCapture(e.pointerId)) {
      indexContainer.releasePointerCapture(e.pointerId);
    }
  }
</script>

<div 
  class="sidebar-index-container" 
  bind:this={indexContainer}
  onpointerdown={onPointerDown}
  onpointermove={onPointerMove}
  onpointerup={onPointerUp}
  onpointercancel={onPointerUp}
  role="slider"
  aria-valuenow={0}
  tabindex="0"
>
  <div class="chars-wrapper" bind:this={charsWrapper}>
    {#each ALL_CHARS as char}
      <div 
        class="index-char" 
        class:active={isScrubbing && char === scrubChar}
      >
        {char}
      </div>
    {/each}
  </div>

  {#if isScrubbing}
    <div 
      class="scrub-callout v-glass" 
      style="top: {bubbleY}px;"
      transition:fade={{ duration: 100 }}
    >
      {scrubChar}
    </div>
  {/if}
</div>

<style>
  .sidebar-index-container {
    position: relative;
    width: 12px;
    border-radius: 12px;
    display: flex;
    flex-direction: column;
    justify-content: center;
    align-items: center;
    touch-action: none;
    user-select: none;
    cursor: pointer;
    z-index: 50;
    margin-left: 12px;
    margin-right: 0px;
    flex-shrink: 0;
  }

  .chars-wrapper {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 18px;
    width: 100%;
  }

  .index-char {
    font-size: 12px;
    font-weight: 700;
    color: var(--text-muted);
    opacity: 0.5;
    display: flex;
    align-items: center;
    justify-content: center;
    height: 12px;
    width: 100%;
  }

  .index-char.active {
    color: var(--text-main);
    opacity: 1;
  }

  .scrub-callout {
    position: absolute;
    right: 24px;
    transform: translateY(-50%);
    width: 40px;
    height: 40px;
    border-radius: 24px;
    display: flex;
    align-items: center;
    justify-content: center;
    font-size: 20px;
    font-weight: 700;
    color: var(--text-main);
    pointer-events: none;
    box-shadow: var(--panel-shadow);
  }
</style>

