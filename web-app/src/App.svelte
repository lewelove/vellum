<script lang="ts">
  import { onMount } from "svelte";
  import { library } from "./library.svelte.ts";
  import { nav, setTab } from "./navigation.svelte.ts";
  
  import HomeView from "./modules/HomeView/HomeView.svelte";
  import ShelvesView from "./modules/ShelvesView/ShelvesView.svelte";
  import QueueView from "./modules/QueueView/QueueView.svelte";

  const tabOrder: Record<string, number> = { home: 1, queue: 2, shelves: 3 };
  let currentTab = $state(nav.activeTab);
  let retentionTab: string | null = $state(null);
  let instantTab: string | null = $state(null);

  $effect(() => {
    if (nav.activeTab !== currentTab) {
      const oldTab = currentTab;
      const newTab = nav.activeTab;
      
      if (tabOrder[newTab] > tabOrder[oldTab]) {
        retentionTab = oldTab;
        instantTab = null;
        setTimeout(() => {
          if (retentionTab === oldTab) retentionTab = null;
        }, 100);
      } else {
        retentionTab = null;
        instantTab = newTab;
        setTimeout(() => {
          if (instantTab === newTab) instantTab = null;
        }, 100);
      }
      
      currentTab = newTab;
    }
  });

  let isHomeActive = $derived(currentTab === 'home');
  let isQueueActive = $derived(currentTab === 'queue');
  let isShelvesActive = $derived(currentTab === 'shelves');

  let isHomeVisible = true;
  let isQueueVisible = $derived(currentTab === 'queue' || retentionTab === 'queue');
  let isShelvesVisible = $derived(currentTab === 'shelves' || retentionTab === 'shelves');

  let isQueueInstant = $derived(instantTab === 'queue');
  let isShelvesInstant = $derived(instantTab === 'shelves');

  let isModalVisible = $derived(!!library.focusedAlbum);

  function handleKeydown(e: KeyboardEvent) {
    if (['INPUT', 'TEXTAREA'].includes(document.activeElement?.tagName ?? "")) return;
    
    const code = e.code;
    const key = e.key;
    
    if (code === 'Space') {
      e.preventDefault();
      e.stopPropagation();
      fetch('/api/toggle-pause', { method: 'POST' }).catch(() => {});
      return;
    }

    if (key === 'Enter' || code === 'NumpadEnter') {
      e.preventDefault();
      e.stopPropagation();
      return;
    }

    const lowerKey = key.toLowerCase();
    
    if (lowerKey === 'escape' && isModalVisible) {
      e.preventDefault();
      e.stopPropagation();
      library.closeFocus();
      return;
    }

    if (lowerKey === '1' || lowerKey === 'h') setTab('home');
    if (lowerKey === '2' || lowerKey === 'q') setTab('queue');
    if (lowerKey === '3' || lowerKey === 's') setTab('shelves');
  }

  function handleFocusIn(e: FocusEvent) {
    if (!e.target) return;
    const tag = (e.target as HTMLElement).tagName;
    if (tag !== 'INPUT' && tag !== 'TEXTAREA') {
      if (e.target !== document.body) {
        document.body.focus({ preventScroll: true });
      }
    }
  }

  onMount(() => {
    library.init();
    window.addEventListener("keydown", handleKeydown, { capture: true });
    window.addEventListener("focusin", handleFocusIn, { capture: true });
    return () => {
      window.removeEventListener("keydown", handleKeydown, { capture: true });
      window.removeEventListener("focusin", handleFocusIn, { capture: true });
    };
  });
</script>

<svelte:head>
  <link rel="stylesheet" href="/api/theme/css?v={library.themeVersion}" />
</svelte:head>

<main tabindex="-1">
  
  <div class="view-layer home" class:visible={isHomeVisible} class:active={isHomeActive} aria-hidden={!isHomeActive}>
    <HomeView />
  </div>

  <div class="view-layer queue" class:visible={isQueueVisible} class:active={isQueueActive} class:instant={isQueueInstant} aria-hidden={!isQueueActive}>
    <QueueView />
  </div>

  <div class="view-layer shelves" class:visible={isShelvesVisible} class:active={isShelvesActive} class:instant={isShelvesInstant} aria-hidden={!isShelvesActive}>
    <ShelvesView />
  </div>

</main>

<style>
  :root {
    --nav-height: 80px;
    --trigger-size: 24px;
  }

  main {
    position: relative;
    width: 100%;
    height: 100%;
    overflow: hidden;
    background-color: var(--background-main);
    outline: none;
  }

  .view-layer {
    position: absolute;
    inset: 0;
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: row;
    overflow: hidden;
    opacity: 0;
    visibility: hidden;
    pointer-events: none;
    transition: opacity 0.0s ease-out, visibility 0.0s ease-out;
  }

  .view-layer.visible {
    opacity: 1;
    visibility: visible;
    transition: opacity 0.0s ease-out, visibility 0.0s ease-out;
  }

  .view-layer.instant {
    transition: none !important;
  }

  .view-layer.active {
    pointer-events: auto;
  }

  .view-layer.home {
    z-index: 1;
    background-color: var(--background-main);
  }

  .view-layer.queue {
    z-index: 2;
    background-color: var(--background-main);
  }

  .view-layer.shelves {
    z-index: 3;
    background-color: var(--background-main);
  }
</style>
