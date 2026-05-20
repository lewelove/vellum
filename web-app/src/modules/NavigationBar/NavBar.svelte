<script lang="ts">
  import { nav, setTab } from "../../navigation.svelte.ts";

  let { variant = "solid" } = $props();
</script>

{#snippet NavButton({ icon, tab }: { icon: string, tab: string })}
  <button 
    class="v-btn-icon nav-button" 
    class:active={nav.activeTab === tab} 
    onclick={() => setTab(tab)}
    title={tab}
  >
    <img src="/{icon}" alt={tab} class="nav-icon" />
  </button>
{/snippet}

<nav class="nav-bar" class:v-glass={variant === 'glass'}>
  <div class="nav-group top">
    {@render NavButton({ icon: "icons/outlined/24px/house.svg", tab: "home" })}
    {@render NavButton({ icon: "icons/outlined/24px/queue_music.svg", tab: "queue" })}
    {@render NavButton({ icon: "icons/outlined/24px/newsstand.svg", tab: "shelves" })}
  </div>
</nav>

<style>
  .nav-bar {
    height: 100%;
    display: flex;
    flex-direction: column;
    justify-content: space-between;
    align-items: center;
    padding: 8px;
    box-sizing: border-box;
    box-shadow: var(--panel-shadow);
    z-index: 100;
    flex-shrink: 0;
  }

  .nav-bar:not(:global(.v-glass)) {
    background-color: var(--background-drawer);
  }
  
  .nav-group {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .nav-button {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    box-shadow: var(--button-shadow-lesser);
    flex-shrink: 0;
    pointer-events: auto;
  }

  .nav-icon {
    width: 20px;
    height: 20px;
  }
</style>
