<script lang="ts">
  import { nav, setTab } from "../../navigation.svelte.ts";
  import { library } from "../../library.svelte.ts";

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

{#snippet SubNavButton({ icon, view, title }: { icon: string, view: "library" | "shelves", title: string })}
  <button 
    class="v-btn-icon nav-button" 
    class:active={library.homeSubView === view} 
    onclick={() => {
      library.homeSubView = view;
      library.refreshView(true);
      library.persistState();
    }}
    {title}
  >
    <img src="/{icon}" alt={title} class="nav-icon" />
  </button>
{/snippet}

<nav class="nav-bar" class:v-glass={variant === 'glass'}>
  <div class="nav-top-section">
    <div class="nav-group top">
      {@render NavButton({ icon: "icons/outlined/24px/house.svg", tab: "home" })}
      {@render NavButton({ icon: "icons/outlined/24px/queue_music.svg", tab: "queue" })}
    </div>

    {#if nav.activeTab === 'home'}
      <div class="nav-separator"></div>
      <div class="nav-group middle">
        {@render SubNavButton({ icon: "icons/outlined/20px/auto_stories.svg", view: "library", title: "Library" })}
        {@render SubNavButton({ icon: "icons/outlined/24px/newsstand.svg", view: "shelves", title: "Shelves" })}
      </div>
    {/if}
  </div>

  <div class="nav-group bottom"></div>
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

  .nav-top-section {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 8px;
    width: 100%;
  }

  .nav-separator {
    width: 100%;
    height: 1px;
    background-color: oklch(100% 0 0 / 0.07);
    margin: 4px 0;
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
