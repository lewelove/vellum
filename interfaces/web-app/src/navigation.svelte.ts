export const nav = $state<{ activeTab: string }>({
  activeTab: 'home'
});

export async function setTab(tab: string) {
  if (nav.activeTab === tab) return;
  nav.activeTab = tab;
  
  const { library } = await import("./library.svelte.ts");
  library.refreshView(false);
  library.persistState();
}
