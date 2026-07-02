export const nav = $state<{ activeTab: string }>({
  activeTab: 'home'
});

export async function setTab(tab: string) {
  if (nav.activeTab === tab) return;
  nav.activeTab = tab;

  const { view } = await import("./library/view.svelte.ts");
  view.refreshView(false);
  view.persistState();
}
