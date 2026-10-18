<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api, type EcoInfo, type EcoRow } from "./api";
  import { i18n, t } from "./i18n.svelte";
  import { ui, toast } from "./store.svelte";

  let rows = $state<EcoRow[]>([]);
  let info = $state<EcoInfo | null>(null);
  let loading = $state(false);
  let query = $state("");

  const meta = $derived(ui.meta!);
  const maxMode = $derived(meta.ecores > 0 ? 2 : 1);
  const shown = $derived(query ? rows.filter((r) => r.name.includes(query.toLowerCase())) : rows);

  async function refresh() {
    loading = true;
    try {
      rows = await api.ecoRows();
    } finally {
      loading = false;
    }
  }

  async function refreshInfo() {
    info = await api.ecoInfo();
  }

  async function setMode(r: EcoRow, mode: number) {
    if (mode === r.mode || mode > maxMode) return;
    const res = await api.ecoSet(r.name, mode);
    r.mode = mode;
    r.eco = mode > 0 ? res.ok : 0;
    let msg = t("eco.msg.set", { name: r.name, mode: t(`eco.mode.${mode}`) });
    if (res.ok + res.bad) msg += " " + t("eco.msg.applied", { ok: res.ok, n: res.ok + res.bad });
    if (res.bad) msg += " " + t("eco.msg.denied");
    if (!res.saved) msg += " " + t("eco.msg.saveFailed");
    if (mode > 0 && info && !info.autostart.enabled) msg += " " + t("eco.msg.autostart");
    toast(msg, res.bad || !res.saved ? "warn" : "ok");
  }

  async function togglePause() {
    if (!info) return;
    await api.setPaused(!info.paused);
    await refreshInfo();
  }

  onMount(() => {
    refresh();
    refreshInfo();
    let un: (() => void) | undefined;
    listen("state-changed", refreshInfo).then((f) => (un = f));
    const timer = setInterval(() => { if (!document.hidden) refreshInfo(); }, 5000);
    return () => { clearInterval(timer); un?.(); };
  });
</script>

<div class="page">
  <header>
    <h1 class="title">{t("eco.title")}</h1>
    <div class="facts">
      <span class="caption">{meta.ecores > 0 ? t("eco.cores", { n: meta.ecores, m: meta.ncpu, mask: meta.emask }) : t("eco.noHybrid")}</span>
      {#if info}
        <span class="caption">·</span>
        <span class="caption">{t("eco.auto")}:
          {#if info.paused}<b class="warn">{t("eco.paused")}</b>{:else}<b class="ok">{t("eco.running")}</b> ({info.method === "wmi" ? t("eco.methodWmi") : t("eco.methodPoll")}){/if}
        </span>
        <button class="btn subtle small" onclick={togglePause}>{info.paused ? t("eco.resume") : t("eco.pause")}</button>
      {/if}
    </div>
    <div class="bar">
      <input class="input search" placeholder={t("eco.search")} bind:value={query} />
      <span class="spacer"></span>
      <button class="btn" onclick={refresh} disabled={loading}>
        <svg viewBox="0 0 16 16" aria-hidden="true" class:spin={loading}><path d="M13.5 8a5.5 5.5 0 1 1-1.6-3.9M13.5 2.5v3h-3" fill="none" stroke="currentColor" stroke-linecap="round" stroke-linejoin="round" /></svg>
        {loading ? t("eco.counting") : t("eco.refresh")}
      </button>
    </div>
  </header>

  <div class="scroll">
    <div class="card table">
      <div class="trow thead">
        <span>{t("eco.col.app")}</span>
        <span>{t("eco.col.mode")}</span>
        <span class="num">{t("eco.col.procs")}</span>
        <span class="num">{t("eco.col.cpu")}</span>
        <span class="num">{t("eco.col.eco")}</span>
      </div>
      {#each shown as r (r.name)}
        <div class="trow" class:on={r.mode > 0}>
          <span class="app" title={r.name}>{r.name}</span>
          <span class="seg" role="radiogroup" aria-label={r.name}>
            {#each [0, 1, 2] as m}
              <button
                class="sbtn"
                class:act={r.mode === m}
                class:m1={m === 1}
                class:m2={m === 2}
                disabled={m > maxMode}
                role="radio"
                aria-checked={r.mode === m}
                onclick={() => setMode(r, m)}>{t(`eco.mode.${m}`)}</button
              >
            {/each}
          </span>
          <span class="num">{r.procs || "—"}</span>
          <span class="num">{r.procs ? `${r.cpu.toLocaleString(i18n.lang, { minimumFractionDigits: 1, maximumFractionDigits: 1 })} %` : ""}</span>
          <span class="num muted">{r.procs ? t("eco.inEco", { e: r.eco, n: r.procs }) : t("eco.notRunning")}</span>
        </div>
      {/each}
      {#if loading && rows.length === 0}
        <div class="empty caption">{t("eco.counting")}</div>
      {/if}
    </div>
  </div>

  <section class="explain card">
    <p>{t("eco.explain")}</p>
    <p class="caption">{t("eco.note")}</p>
  </section>
</div>

<style>
  .page { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  header { padding: 20px 28px 12px; }
  .facts { display: flex; align-items: center; gap: 8px; margin-top: 6px; flex-wrap: wrap; }
  .facts b { font-weight: 600; }
  .ok { color: var(--success); }
  .warn { color: var(--caution); }
  .bar { display: flex; align-items: center; gap: 10px; margin-top: 12px; }
  .search { width: 280px; }
  .spacer { flex: 1; }
  .btn.small { min-height: 24px; padding: 1px 8px; font-size: 12px; }
  .spin { animation: spin 1s linear infinite; }
  @keyframes spin { to { transform: rotate(360deg); } }

  .scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 0 28px 12px; }
  .table { padding: 0 0 4px; }
  .trow { display: grid; grid-template-columns: minmax(110px, 1fr) auto 64px 76px 92px; gap: 12px; align-items: center; padding: 6px 16px; min-height: 44px; }
  .trow + .trow { border-top: 1px solid var(--divider); }
  .thead { position: sticky; top: 0; z-index: 1; min-height: 36px; font-size: 12px; color: var(--text-2); background: var(--flyout); border-radius: var(--radius-l) var(--radius-l) 0 0; }
  .trow.on .app { font-weight: 600; }
  .app { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .num { text-align: right; font-variant-numeric: tabular-nums; }
  .empty { padding: 20px; text-align: center; }

  .seg { display: inline-flex; padding: 2px; border-radius: var(--radius); background: var(--subtle-press); border: 1px solid var(--control-stroke); }
  .sbtn { flex: 1; height: 26px; padding: 0 9px; border: 0; border-radius: 3px; background: transparent; color: var(--text-2); font-size: 12px; cursor: default; white-space: nowrap; }
  .sbtn:hover:not(:disabled):not(.act) { background: var(--subtle-hover); color: var(--text); }
  .sbtn:disabled { color: var(--text-3); opacity: 0.5; }
  .sbtn.act { background: var(--control); color: var(--text); box-shadow: 0 0 0 1px var(--control-stroke); font-weight: 600; }
  .sbtn.act.m1 { color: var(--success); }
  .sbtn.act.m2 { color: var(--accent); }

  .explain { margin: 0 28px 16px; padding: 10px 16px; }
  .explain p { margin: 0; color: var(--text-2); }
  .explain p + p { margin-top: 6px; }
</style>
