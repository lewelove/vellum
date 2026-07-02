<script lang="ts">
  import { view } from "../../library/view.svelte.ts";
  import { collection } from "../../library/collection.svelte.ts";

  let activeShelf = $derived(view.activeShelf || (collection.manifest.shelves_order && collection.manifest.shelves_order[0]) || Object.keys(collection.availableShelves)[0]);

  function selectShelf(key: string) {
    view.setShelf(key);
  }
</script>

<div class="sidebar-container">
  <div class="sidebar-scroll">
    <div class="v-scroll-fade-top"></div>
    {#each collection.shelvesList as shelf}
      <button 
        class="sidebar-item" 
        class:active={activeShelf === shelf.key} 
        onclick={() => selectShelf(shelf.key)}
      >
        <span class="v-truncate label" title={shelf.label}>{shelf.label}</span>
      </button>
    {/each}
    <div class="scroll-spacer"></div>
    <div class="v-scroll-fade-bottom"></div>
  </div>
</div>

<style>
  .sidebar-container {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background-color: var(--background-drawer); 
    padding: 6px 12px; 
    box-sizing: border-box;
  }

  .sidebar-scroll {
    position: relative;
    flex: 1;
    overflow-y: scroll;
    padding: 0;
    min-height: 0;
    scrollbar-width: none;
    -ms-overflow-style: none;
    border-bottom: 1px solid rgba(255, 255, 255, 0.05);
  }

  .sidebar-scroll::-webkit-scrollbar {
    display: none;
  }

  .scroll-spacer {
    height: 12px;
    flex-shrink: 0;
  }

  .sidebar-item {
    display: flex;
    justify-content: space-between;
    align-items: center;
    width: 100%;
    background-color: transparent;
    border: none;
    padding: 6px 12px;
    margin-bottom: 2px;
    cursor: default;
    color: var(--text-muted);
    font-family: var(--font-stack);
    font-size: 14px;
    text-align: left;
    outline: none;
    border-radius: 8px;
    box-sizing: border-box;
    user-select: none;
  }

  .sidebar-item:hover {
    background-color: rgba(255, 255, 255, 0.03);
    color: var(--text-main);
    cursor: pointer;
  }

  .sidebar-item.active {
    background-color: rgba(255, 255, 255, 0.05);
    color: var(--text-main);
  }

  .label {
    flex: 1;
    margin-right: 8px;
  }
</style>
