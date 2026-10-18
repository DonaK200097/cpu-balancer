<script lang="ts">
  import { onMount, tick } from "svelte";
  import { api, checkUpdate } from "./lib/api";
  import { i18n, t } from "./lib/i18n.svelte";
  import { ui } from "./lib/store.svelte";
  import Editor from "./lib/Editor.svelte";
  import Eco from "./lib/Eco.svelte";
  import Settings from "./lib/Settings.svelte";

  let visited = $state<Record<string, boolean>>({ editor: true });

  function go(p: typeof ui.page) {
    ui.page = p;
    visited[p] = true;
  }

  onMount(async () => {
    const meta = await api.meta();
    ui.meta = meta;
    i18n.lang = meta.lang;
    document.documentElement.lang = meta.lang;
    if (meta.accent) document.documentElement.style.setProperty("--accent-base", meta.accent);
    if (!meta.mica) document.body.classList.add("no-mica");
    await tick();
    // окно показывается после первого кадра — без белой вспышки
    requestAnimationFrame(() => requestAnimationFrame(async () => {
      ui.startupMs = await api.uiReady();
      if (meta.check_updates) setTimeout(async () => (ui.update = await checkUpdate(meta.version)), 3000);
    }));
  });

  const nav = [
    { id: "editor", icon: "M3 4.5h10M3 8h10M3 11.5h10M6 3v3M10 6.5v3M5 10v3" },
    { id: "eco", icon: "M8 14c-3-1.5-5-4-5-7.5C5.5 6 7 5 8 2c1 3 2.5 4 5 4.5 0 3.5-2 6-5 7.5zM8 14V7" },
  ] as const;
</script>

<div class="shell" class:ready={ui.meta}>
  <nav>
    {#each nav as n}
      <button class="nitem" class:act={ui.page === n.id} onclick={() => go(n.id)} title={t(`nav.${n.id}`)}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d={n.icon} fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" /></svg>
        {t(`nav.${n.id}`)}
      </button>
    {/each}
    <span class="grow"></span>
    <button class="nitem" class:act={ui.page === "settings"} onclick={() => go("settings")} title={t("nav.settings")}>
      <svg viewBox="0 0 16 16" aria-hidden="true"><circle cx="8" cy="8" r="2" fill="none" stroke="currentColor" /><path d="M8 1.5v2M8 12.5v2M1.5 8h2M12.5 8h2M3.4 3.4l1.4 1.4M11.2 11.2l1.4 1.4M3.4 12.6l1.4-1.4M11.2 4.8l1.4-1.4" stroke="currentColor" stroke-linecap="round" /></svg>
      {t("nav.settings")}
    </button>
  </nav>

  <main>
    {#if ui.update}
      <div class="infobar">
        <span class="body-strong">{t("update.available", { v: ui.update.version })}</span>
        <span class="grow"></span>
        <button class="btn accent" onclick={() => api.openUrl(ui.update!.url)}>{t("update.download")}</button>
        <button class="btn subtle" onclick={() => (ui.update = null)} aria-label="×">✕</button>
      </div>
    {/if}
    {#if ui.meta}
      <div class="pane" class:hidden={ui.page !== "editor"}><Editor /></div>
      {#if visited.eco}<div class="pane" class:hidden={ui.page !== "eco"}><Eco /></div>{/if}
      {#if visited.settings}<div class="pane" class:hidden={ui.page !== "settings"}><Settings /></div>{/if}
    {/if}
    {#if ui.toast}
      <div class="toast {ui.toast.kind}" role="status">
        <span class="ticon">{ui.toast.kind === "ok" ? "✓" : "!"}</span>
        <span>{ui.toast.text}</span>
        <button class="btn subtle x" onclick={() => (ui.toast = null)} aria-label="×">✕</button>
      </div>
    {/if}
  </main>
</div>

<style>
  .shell { display: flex; height: 100%; opacity: 0; }
  .shell.ready { opacity: 1; }
  nav { width: 230px; flex: none; display: flex; flex-direction: column; gap: 2px; padding: 8px 6px 10px; }
  .nitem {
    position: relative; display: flex; align-items: center; gap: 14px; height: 36px; padding: 0 12px; border: 0; border-radius: var(--radius);
    background: transparent; text-align: left; cursor: default;
  }
  .nitem svg { width: 16px; height: 16px; flex: none; }
  .nitem:hover { background: var(--subtle-hover); }
  .nitem.act { background: var(--subtle-hover); }
  .nitem.act::before { content: ""; position: absolute; left: 0; top: 10px; height: 16px; width: 3px; border-radius: 2px; background: var(--accent); }
  .grow { flex: 1; }
  /* узкое окно: только значки, как свёрнутая NavigationView */
  @media (max-width: 1000px) {
    nav { width: 52px; }
    .nitem :global(.lbl) { display: none; }
    .nitem { font-size: 0; justify-content: center; padding: 0; }
  }
  main {
    position: relative; flex: 1; min-width: 0; display: flex; flex-direction: column;
    background: var(--card); border-top-left-radius: var(--radius-l); border: 1px solid var(--card-stroke); border-right: 0; border-bottom: 0;
  }
  .pane { flex: 1; min-height: 0; }
  .pane.hidden { display: none; }
  .infobar { display: flex; align-items: center; gap: 12px; margin: 12px 28px 0; padding: 8px 8px 8px 16px; border-radius: var(--radius-l); background: color-mix(in srgb, var(--accent) 14%, transparent); border: 1px solid var(--card-stroke); }
  .toast {
    position: absolute; right: 28px; bottom: 72px; z-index: 150; max-width: 560px; display: flex; align-items: flex-start; gap: 10px;
    padding: 10px 8px 10px 14px; border-radius: var(--radius-l); background: var(--flyout); border: 1px solid var(--control-stroke);
    box-shadow: var(--shadow-flyout); animation: rise 0.2s cubic-bezier(0.2, 0, 0, 1);
  }
  @keyframes rise { from { opacity: 0; transform: translateY(12px); } }
  .ticon { width: 18px; height: 18px; border-radius: 9px; display: grid; place-items: center; font-size: 11px; font-weight: 700; flex: none; margin-top: 1px; color: #fff; }
  .toast.ok .ticon { background: var(--success); }
  .toast.warn .ticon { background: var(--caution); color: #000; }
  .toast.err .ticon { background: var(--critical); color: #000; }
  .x { min-height: 22px; padding: 0 6px; font-size: 11px; color: var(--text-2); }
</style>
