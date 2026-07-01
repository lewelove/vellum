<script>
  import { nav, setTab } from "../../navigation.svelte.js";

  let { icon, tab } = $props();
  let isActive = $derived(nav.activeTab === tab);
</script>

<button 
  class="v-btn-icon nav-button" 
  class:active={isActive} 
  onclick={() => setTab(tab)}
  title={tab}
>
  <img src="/{icon}" alt={tab} class="nav-icon" />
</button>

<style>
  .nav-button {
    width: 36px;
    height: 36px;
    border-radius: 16px;
    box-shadow: var(--button-shadow-lesser);
    flex-shrink: 0;
    pointer-events: auto;
  }

  .nav-icon {
    width: 24px;
    height: 24px;
  }
</style>
