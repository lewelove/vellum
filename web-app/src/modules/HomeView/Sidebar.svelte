<script lang="ts">
  import { library } from "../../library.svelte.ts";
  import SidebarIndex from "./SidebarIndex.svelte";

  let isLibraryMenuOpen = $state(false);
  let isLibraryFilterMenuOpen = $state(false);
  let isSortMenuOpen = $state(false);
  let isGroupMenuOpen = $state(false);
  let scrollContainer: HTMLDivElement | null = $state(null);

  let activeLibraryDef = $derived(library.availableLibraries[library.activeLibrary] || {});
  let allowedFilters = $derived(activeLibraryDef.allowed_filters || []);
  let showFilterDropdown = $derived(allowedFilters.length > 1);

  let libraryLabel = $derived(library.availableLibraries[library.activeLibrary]?.label || "Unknown");
  let filterLabel = $derived(library.availableFilters[library.activeLibraryFilter || ""]?.label || "Unknown");
  let groupLabel = $derived(library.availableFacets[library.activeSidebarGrouper]?.label || "Unknown");
  let sortLabel = $derived(library.availableOrders[library.userSortPreference]?.label || "Unknown");

  let items = $derived(library.getSidebarGroup(library.activeSidebarGrouper));

  let isReverse = $derived(library.userSortOrder === "reverse");

  let activeGrouperDef = $derived(library.availableFacets[library.activeSidebarGrouper] || {});
  let showIndex = $derived(activeGrouperDef.index === true);
  let showCount = $derived(activeGrouperDef.count === true);

  function toggleLibraryMenu() {
    isLibraryMenuOpen = !isLibraryMenuOpen;
    if (isLibraryMenuOpen) {
      isLibraryFilterMenuOpen = false;
      isSortMenuOpen = false;
      isGroupMenuOpen = false;
    }
  }

  function toggleLibraryFilterMenu() {
    isLibraryFilterMenuOpen = !isLibraryFilterMenuOpen;
    if (isLibraryFilterMenuOpen) {
      isLibraryMenuOpen = false;
      isSortMenuOpen = false;
      isGroupMenuOpen = false;
    }
  }

  function toggleSortMenu() {
    isSortMenuOpen = !isSortMenuOpen;
    if (isSortMenuOpen) {
      isLibraryMenuOpen = false;
      isLibraryFilterMenuOpen = false;
      isGroupMenuOpen = false;
    }
  }

  function toggleGroupMenu() {
    isGroupMenuOpen = !isGroupMenuOpen;
    if (isGroupMenuOpen) {
      isLibraryMenuOpen = false;
      isLibraryFilterMenuOpen = false;
      isSortMenuOpen = false;
    }
  }

  function selectLibrary(key: string) {
    library.setLibrary(key);
    isLibraryMenuOpen = false;
  }

  function selectOrder(key: string) {
    library.setUserSort(key);
    isSortMenuOpen = false;
  }

  function selectGrouper(key: string) {
    library.setSidebarGrouper(key);
    isGroupMenuOpen = false;
  }

  function toggleDirection() {
    library.toggleSortOrder();
  }
</script>

{#snippet Item({ index, label, count, active, onclick }: { index: number, label: string, count: number, active: boolean, onclick: () => void })}
  <button id="sidebar-item-{index}" class="sidebar-item" class:active {onclick}>
    <span class="v-truncate label" title={label}>{label}</span>
    {#if showCount}
      <span class="count">{count}</span>
    {/if}
  </button>
{/snippet}

<div class="sidebar-container">

  <div class="sidebar-controls">
    <div class="control-row">
      <div class="button-wrapper flex-grow">
        <button class="v-btn-icon sidebar-btn" onclick={toggleLibraryMenu} class:active={isLibraryMenuOpen} title="Library">
          <img src="icons/outlined/20px/auto_stories.svg" alt="" class="start-icon" />
          <span class="v-truncate btn-label">{libraryLabel}</span>
          <img 
            src={isLibraryMenuOpen ? "icons/outlined/24px/arrow_drop_up.svg" : "icons/outlined/24px/arrow_drop_down.svg"}  
            class="end-icon" 
            alt="" 
          />
        </button>
    
        {#if isLibraryMenuOpen}
          <div class="control-menu v-panel">
            {#each library.librariesList as lib}
              <button 
                class="menu-item" 
                class:selected={library.activeLibrary === lib.key}
                onclick={() => selectLibrary(lib.key)}
              >
                {lib.label}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    {#if showFilterDropdown}
      <div class="control-row">
        <div class="button-wrapper flex-grow">
          <button class="v-btn-icon sidebar-btn" onclick={toggleLibraryFilterMenu} class:active={isLibraryFilterMenuOpen} title="Filter">
            <img src="icons/outlined/24px/format_list_bulleted.svg" alt="" class="start-icon" />
            <span class="v-truncate btn-label">{filterLabel}</span>
            <img 
              src={isLibraryFilterMenuOpen ? "icons/outlined/24px/arrow_drop_up.svg" : "icons/outlined/24px/arrow_drop_down.svg"}  
              class="end-icon" 
              alt="" 
            />
          </button>
      
          {#if isLibraryFilterMenuOpen}
            <div class="control-menu v-panel">
              {#each allowedFilters as fKey}
                <button 
                  class="menu-item" 
                  class:selected={library.activeLibraryFilter === fKey}
                  onclick={() => {
                    library.setLibraryFilter(fKey);
                    isLibraryFilterMenuOpen = false;
                  }}
                >
                  {library.availableFilters[fKey]?.label || fKey}
                </button>
              {/each}
            </div>
          {/if}
        </div>
      </div>
    {/if}

    <div class="control-row">
      <div class="button-wrapper flex-grow">
        <button class="v-btn-icon sidebar-btn" onclick={toggleGroupMenu} class:active={isGroupMenuOpen} title="Group By">
          <img src="icons/outlined/20px/stack_group.svg" alt="" class="start-icon" />
          <span class="v-truncate btn-label">{groupLabel}</span>
          <img 
            src={isGroupMenuOpen ? "icons/outlined/24px/arrow_drop_up.svg" : "icons/outlined/24px/arrow_drop_down.svg"}  
            class="end-icon" 
            alt="" 
          />
        </button>
    
        {#if isGroupMenuOpen}
          <div class="control-menu v-panel">
            {#each library.visibleFacets as {key, label}}
              <button 
                class="menu-item" 
                class:selected={library.activeSidebarGrouper === key}
                onclick={() => selectGrouper(key)}
              >
                {label}
              </button>
            {/each}
          </div>
        {/if}
      </div>
    </div>

    <div class="control-row">
      <div class="button-wrapper flex-grow">
        <button class="v-btn-icon sidebar-btn" onclick={toggleSortMenu} class:active={isSortMenuOpen} title="Sort By">
          <img src="icons/outlined/20px/swap_vert.svg" alt="" class="start-icon" />
          <span class="v-truncate btn-label">{sortLabel}</span>
          <img 
            src={isSortMenuOpen ? "icons/outlined/24px/arrow_drop_up.svg" : "icons/outlined/24px/arrow_drop_down.svg"} 
            class="end-icon" 
            alt="" 
          />
        </button>

        {#if isSortMenuOpen}
          <div class="control-menu v-panel">
            {#each library.visibleOrders as {key, label}}
              <button 
                class="menu-item" 
                class:selected={library.userSortPreference === key}
                onclick={() => selectOrder(key)}
              >
                {label}
              </button>
            {/each}
          </div>
        {/if}
      </div>
      
      <button class="v-btn-icon dir-btn" onclick={toggleDirection} title={isReverse ? "Reverse Order" : "Default Order"}>
        <img 
          class:mirrored={isReverse}
          src="/icons/outlined/24px/arrow_shape_up_stack.svg" 
          alt="Direction" 
        />
      </button>
    </div>
  </div>

  <div class="sidebar-body">
    <div class="sidebar-scroll" bind:this={scrollContainer}>
      <div class="v-scroll-fade-top"></div>
      {#each items as item, i}
        {@render Item({
          index: i,
          label: item.label,
          count: item.count,
          active: library.activeFilter.key === library.activeSidebarGrouper && library.activeFilter.val === item.value,
          onclick: () => library.applyFilter(library.activeSidebarGrouper, item.value)
        })}
      {/each}
      <div class="scroll-spacer"></div>
      <div class="v-scroll-fade-bottom"></div>
    </div>
    
    {#if items.length > 0 && showIndex}
      <SidebarIndex {items} container={scrollContainer} />
    {/if}
  </div>
</div>

<style>
  .sidebar-container {
    width: 100%;
    height: 100%;
    display: flex;
    flex-direction: column;
    background-color: var(--background-drawer); 
    padding: 10px; 
    box-sizing: border-box;
    font-family: var(--font-stack);
  }

  .sidebar-controls {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding-bottom: 10px;
    border-bottom: 1px solid oklch(100% 0 0 / 0.05);
    margin-bottom: 0px;
    flex-shrink: 0;
  }

  .control-row {
    display: flex;
    gap: 8px;
    width: 100%;
  }

  .button-wrapper {
    position: relative;
  }

  .flex-grow {
    flex: 1;
    min-width: 0;
  }

  .sidebar-btn {
    width: 100%;
    height: 36px;
    padding: 0 8px;
    justify-content: space-between;
    border-radius: 10px;
    color: var(--text-muted);
    font-size: 14px;
    font-family: var(--font-stack);
    font-weight: 500;
  }

  .dir-btn {
    width: 36px;
    height: 36px;
    border-radius: 10px;
    flex-shrink: 0;
  }

  .dir-btn img.mirrored {
    transform: scaleY(-1);
  }

  .start-icon {
    width: 20px;
    height: 20px;
    flex-shrink: 0;
  }

  .end-icon {
    width: 20px;
    height: 20px;
    margin-left: 4px;
    flex-shrink: 0;
  }

  .btn-label {
    flex: 1;
    padding-left: 7px;
    text-align: left;
    font-family: var(--font-stack);
  }

  .control-menu {
    position: absolute;
    top: 100%;
    left: 0;
    width: 100%;
    margin-top: 6px;
    z-index: 50;
    border: 2px solid oklch(100% 0 0 / 0.05);
    border-radius: 12px;
    padding: 4px;
    box-sizing: border-box;
  }

  .menu-item {
    display: block;
    width: 100%;
    text-align: left;
    padding: 6px 10px;
    margin-bottom: 2px;
    background: none;
    border: none;
    color: var(--text-muted);
    font-size: 14px;
    font-family: var(--font-stack);
    cursor: pointer;
    border-radius: 8px;
    box-sizing: border-box;
  }

  .menu-item:hover {
    background-color: oklch(100% 0 0 / 0.03);
    color: var(--text-main);
  }

  .menu-item.selected {
    color: var(--text-main);
    background-color: oklch(100% 0 0 / 0.05);
  }

  .sidebar-body {
    position: relative;
    flex: 1;
    min-height: 0;
    display: flex;
    flex-direction: row;
  }

  .sidebar-scroll {
    position: relative;
    flex: 1;
    overflow-y: scroll;
    padding: 0;
    min-height: 0;
    scrollbar-width: none;
    -ms-overflow-style: none;
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
    padding: 6px 10px;
    margin-bottom: 2px;
    cursor: pointer;
    color: var(--text-muted);
    font-size: 14px;
    font-family: var(--font-stack);
    text-align: left;
    border-radius: 8px;
    box-sizing: border-box;
  }

  .sidebar-item:hover {
    background-color: oklch(100% 0 0 / 0.03);
    color: var(--text-main);
  }

  .sidebar-item.active {
    background-color: oklch(100% 0 0 / 0.05);
    color: var(--text-main);
  }

  .label {
    flex: 1;
    margin-right: 8px;
    font-family: var(--font-stack);
  }

  .count {
    margin-left: 8px;
    font-size: 12px;
    opacity: 0.5;
    font-family: var(--font-mono);
  }
</style>
