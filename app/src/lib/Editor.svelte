<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api, type Cell, type Control, type CtlView, type EditorView, type Scheme } from "./api";
  import { t } from "./i18n.svelte";
  import { ui, toast, controlDef } from "./store.svelte";
  import Slider from "./Slider.svelte";
  import Select from "./Select.svelte";

  let schemes = $state<Scheme[]>([]);
  let view = $state<EditorView | null>(null);
  let onAc = $state(true);
  let selId = $state("eppP");
  let selCol = $state(0);
  let presetsOpen = $state(false);
  let confirm = $state<{ resolve: (a: "save" | "drop" | "stay") => void } | null>(null);

  const byId = $derived(new Map((view?.controls ?? []).map((c) => [c.id, c])));
  const liveCol = $derived(onAc ? 0 : 1);
  const colName = (c: number) => t(c === 0 ? "editor.ac" : "editor.dc");

  function unit(u: string) {
    return u === "MHz" ? t("unit.MHz") : u;
  }

  function cellText(def: Control, cell: Cell): string {
    if (def.kind === "slider") {
      if (cell.value == null) return "—";
      if (def.zero && cell.value === 0) return t("editor.noLimit");
      return `${cell.value} ${unit(def.unit)}`.trim();
    }
    if (def.kind === "enum") {
      const k = `opt.${def.id}.${cell.value}`;
      return def.opts.includes(cell.value ?? NaN) ? t(k) : t("editor.ownVal", { v: cell.value ?? "—" });
    }
    if (cell.mode < 0) return t("editor.own");
    return t(`opt.${def.id}.${def.opts[cell.mode].id}`);
  }

  function options(def: Control) {
    if (def.kind === "enum") return def.opts.map((v) => ({ value: v, label: t(`opt.${def.id}.${v}`) }));
    if (def.kind === "mode") return def.opts.map((o, i) => ({ value: i, label: t(`opt.${def.id}.${o.id}`) }));
    return [];
  }

  async function refreshSchemes() {
    schemes = await api.schemes();
  }

  async function open(guid: string) {
    view = await api.openScheme(guid);
  }

  async function ask(): Promise<"save" | "drop" | "stay"> {
    return new Promise((resolve) => (confirm = { resolve }));
  }
  function answer(a: "save" | "drop" | "stay") {
    confirm?.resolve(a);
    confirm = null;
  }

  async function switchScheme(guid: string) {
    if (view && view.pending > 0) {
      const a = await ask();
      if (a === "stay") return;
      if (a === "save") await save();
    }
    await open(guid);
  }

  async function activate() {
    if (!view) return;
    schemes = await api.setActive(view.scheme);
    view = await api.reloadScheme();
    toast(t("msg.activated", { name: view.name }));
  }

  async function setCtl(id: string, col: number, value: number) {
    selId = id;
    selCol = col;
    view = await api.setControl(id, col, value);
  }

  async function save() {
    if (!view || view.pending === 0) return toast(t("msg.nothing"));
    const r = await api.save();
    view = r.view;
    let msg = t("msg.saved", { n: r.result.saved });
    if (r.result.failed.length) msg += " " + t("msg.failed", { list: r.result.failed.join(", ") });
    if (!r.result.active) msg += " " + t("msg.notActive");
    toast(msg, r.result.failed.length ? "err" : r.result.active ? "ok" : "warn");
  }

  async function discard() {
    view = await api.discard();
    toast(t("msg.discarded"));
  }

  async function restore() {
    view = await api.restoreBackup();
    toast(t("msg.restored"));
  }

  async function preset(id: string) {
    presetsOpen = false;
    view = await api.applyPreset(id);
    toast(t("msg.preset", { name: t(`preset.${id}.name`) }));
  }

  async function copy(col: number) {
    view = await api.copyControl(selId, col);
    toast(t("msg.copied"));
  }

  // Схему могли переключить G-Helper/Armoury или меню трея — перечитываем при возврате в окно
  async function sync() {
    if (document.hidden) return;
    onAc = await api.powerStatus();
    await refreshSchemes();
    if (view) view = await api.reloadScheme();
  }

  export function hasPending() {
    return (view?.pending ?? 0) > 0;
  }

  function keys(e: KeyboardEvent) {
    if (ui.page !== "editor" || confirm) return;
    if (e.ctrlKey && (e.key === "s" || e.key === "ы")) { e.preventDefault(); save(); }
    else if (e.ctrlKey && (e.key === "z" || e.key === "я") && !(e.target as HTMLElement).closest("input")) { e.preventDefault(); discard(); }
    else if (!e.ctrlKey && !e.altKey && /^[1-9]$/.test(e.key) && !(e.target as HTMLElement).closest("input")) {
      const p = ui.meta?.presets[Number(e.key) - 1];
      if (p) preset(p.id);
    }
  }

  onMount(() => {
    let timer: ReturnType<typeof setInterval>;
    let un: (() => void) | undefined;
    (async () => {
      onAc = await api.powerStatus();
      await refreshSchemes();
      const act = schemes.find((s) => s.active) ?? schemes[0];
      if (act) await open(act.guid);
      un = await listen("state-changed", sync);
    })();
    timer = setInterval(async () => {
      if (!document.hidden) onAc = await api.powerStatus();
    }, 5000);
    window.addEventListener("focus", sync);
    window.addEventListener("keydown", keys);
    return () => {
      clearInterval(timer);
      window.removeEventListener("focus", sync);
      window.removeEventListener("keydown", keys);
      un?.();
    };
  });

  const sel = $derived(byId.get(selId));
  const selDef = $derived(controlDef(selId));
</script>

<div class="page">
  <header>
    <h1 class="title">{t("nav.editor")}</h1>
    <div class="bar">
      <Select
        width="280px"
        options={schemes.map((s) => ({ value: s.guid, label: s.name, hint: s.active ? "● " + t("editor.active") : "" }))}
        value={view?.scheme ?? null}
        display={t("editor.loading")}
        onchange={(g) => switchScheme(String(g))}
      />
      {#if view}
        {#if view.active}
          <span class="badge ok"><span class="dotlive"></span>{t("editor.active")}</span>
        {:else}
          <button class="btn" onclick={activate}>{t("editor.makeActive")}</button>
        {/if}
      {/if}
      <span class="spacer"></span>
      <span class="power" title={t("editor.power")}>
        <svg viewBox="0 0 16 16" aria-hidden="true">
          {#if onAc}
            <path d="M9.5 1.5 4 9h4l-1.5 5.5L12 7H8z" fill="currentColor" />
          {:else}
            <rect x="1.5" y="4.5" width="11" height="7" rx="1.5" fill="none" stroke="currentColor" /><rect x="13" y="6.5" width="1.5" height="3" rx=".5" fill="currentColor" /><rect x="3" y="6" width="5" height="4" rx=".5" fill="currentColor" />
          {/if}
        </svg>
        {t("editor.power")}: <b>{onAc ? t("editor.ac") : t("editor.dc")}</b>
      </span>
    </div>
  </header>

  <div class="scroll">
    <div class="grid colhead">
      <div class="caption">{t("editor.setting")}</div>
      {#each [0, 1] as c}
        <div class="colname" class:live={c === liveCol}>
          {colName(c)}{#if c === liveCol}<span class="now">●</span>{/if}
        </div>
      {/each}
    </div>

    {#if view}
      {#each ui.meta?.groups ?? [] as g}
        <h2 class="group">{t(`group.${g.id}`)}</h2>
        <div class="card rows">
          {#each g.controls as def}
            {@const cv = byId.get(def.id) as CtlView}
            {#if cv}
              <div class="grid row" class:sel={selId === def.id} role="presentation" onpointerdown={() => (selId = def.id)}>
                <div class="name">
                  <span class="chg" class:on={cv.changed} title={cv.changed ? "*" : ""}></span>
                  <span>{t(`ctl.${def.id}.name`)}</span>
                </div>
                {#each [0, 1] as c}
                  <div class="cell" class:live={c === liveCol}>
                    {#if !cv.available}
                      <span class="na">{t("editor.na")}</span>
                    {:else if def.kind === "slider"}
                      <Slider
                        value={cv.cur[c].value}
                        min={def.min}
                        max={def.max}
                        step={def.step}
                        fine={def.fine}
                        big={def.big}
                        zero={def.zero}
                        marker={cv.backup[c].value}
                        label={`${t(`ctl.${def.id}.name`)} — ${colName(c)}`}
                        onchange={(v) => setCtl(def.id, c, v)}
                        onfocus={() => { selId = def.id; selCol = c; }}
                      />
                      <span class="val">{cellText(def, cv.cur[c])}</span>
                    {:else}
                      <Select
                        width="100%"
                        options={options(def)}
                        value={def.kind === "mode" ? (cv.cur[c].mode >= 0 ? cv.cur[c].mode : null) : cv.cur[c].value}
                        display={cellText(def, cv.cur[c])}
                        onchange={(v) => setCtl(def.id, c, Number(v))}
                        onfocus={() => { selId = def.id; selCol = c; }}
                      />
                    {/if}
                  </div>
                {/each}
              </div>
            {/if}
          {/each}
        </div>
      {/each}
      <p class="caption foot">{t("editor.hint")}<br />{t("editor.gHelper")}</p>
    {/if}
  </div>

  <section class="info card">
    {#if sel && selDef}
      <div class="info-text">
        <div class="info-head">
          <span class="body-strong">{t(`ctl.${selDef.id}.name`)}</span>
          {#if sel.available}
            <span class="caption">{t("editor.was", { ac: cellText(selDef, sel.backup[0]), dc: cellText(selDef, sel.backup[1]) })}</span>
          {/if}
        </div>
        <p class="help">{t(`ctl.${selDef.id}.help`)}</p>
      </div>
      {#if sel.available}
        <div class="copy">
          <button class="btn subtle small" onclick={() => copy(0)} title={t("editor.copyTo", { col: colName(1) })}>{colName(0)} → {colName(1)}</button>
          <button class="btn subtle small" onclick={() => copy(1)} title={t("editor.copyTo", { col: colName(0) })}>{colName(1)} → {colName(0)}</button>
        </div>
      {/if}
    {/if}
  </section>

  <footer class="cmdbar">
    <div class="presets">
      <button class="btn" onclick={() => (presetsOpen = !presetsOpen)} aria-expanded={presetsOpen}>
        <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M8 1.5l1.7 3.9 4.2.4-3.2 2.8 1 4.1L8 10.5l-3.7 2.2 1-4.1L2.1 5.8l4.2-.4z" fill="none" stroke="currentColor" stroke-linejoin="round" /></svg>
        {t("editor.presets")}
        <svg viewBox="0 0 16 16" aria-hidden="true" class="chev"><path d="M3.5 10 8 5.5 12.5 10" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" /></svg>
      </button>
      {#if presetsOpen}
        <div class="backdrop" role="presentation" onpointerdown={() => (presetsOpen = false)}></div>
        <div class="menu">
          {#each ui.meta?.presets ?? [] as p, i}
            <button class="mitem" onclick={() => preset(p.id)}>
              <span class="num">{i + 1}</span>
              <span class="mtext"><span>{t(`preset.${p.id}.name`)}</span><span class="caption">{t(`preset.${p.id}.desc`)}</span></span>
            </button>
          {/each}
          <div class="caption mnote">{t("preset.note")}</div>
        </div>
      {/if}
    </div>
    <span class="spacer"></span>
    {#if view && view.pending > 0}<span class="caption pending">{t("editor.pending", { n: view.pending })}</span>{/if}
    <button class="btn" onclick={discard} disabled={!view || view.pending === 0}>{t("editor.discard")}</button>
    <button class="btn" onclick={restore} disabled={!view} title={t("editor.restoreTip")}>{t("editor.restore")}</button>
    <button class="btn accent" onclick={save} disabled={!view || view.pending === 0}>{t("editor.save")}</button>
  </footer>
</div>

{#if confirm}
  <div class="overlay">
    <div class="dialog" role="dialog" aria-modal="true">
      <div class="dbody">
        <h2 class="subtitle">{t("confirm.title")}</h2>
        <p>{t("confirm.text", { name: view?.name ?? "" })}</p>
      </div>
      <div class="dfoot">
        <button class="btn accent" onclick={() => answer("save")}>{t("confirm.save")}</button>
        <button class="btn" onclick={() => answer("drop")}>{t("confirm.drop")}</button>
        <button class="btn" onclick={() => answer("stay")}>{t("confirm.stay")}</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .page { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  header { padding: 20px 28px 12px; }
  .bar { display: flex; align-items: center; gap: 10px; margin-top: 14px; }
  .spacer { flex: 1; }
  .badge { display: inline-flex; align-items: center; gap: 6px; padding: 2px 10px; border-radius: 12px; font-size: 12px; background: var(--subtle-hover); }
  .badge.ok { color: var(--success); }
  .dotlive { width: 6px; height: 6px; border-radius: 3px; background: currentColor; }
  .power { display: inline-flex; align-items: center; gap: 6px; color: var(--text-2); }
  .power b { color: var(--text); font-weight: 600; }
  .power svg { width: 16px; height: 16px; color: var(--accent); }

  .scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 0 28px 12px; }
  .grid { display: grid; grid-template-columns: minmax(190px, 250px) 1fr 1fr; gap: 16px; align-items: center; }
  .colhead { position: sticky; top: 0; z-index: 2; padding: 8px 17px 6px; }
  .colname { font-weight: 600; color: var(--text-2); display: flex; align-items: center; gap: 6px; }
  .colname.live { color: var(--accent); }
  .now { font-size: 9px; }
  .group { font-size: 14px; line-height: 20px; margin: 14px 0 6px 2px; }
  .rows { padding: 2px 0; }
  .row { padding: 4px 16px; min-height: 48px; border-radius: 6px; margin: 0 2px; }
  .row + .row { border-top: 1px solid var(--divider); }
  .row.sel { background: var(--subtle-hover); }
  .name { display: flex; align-items: center; gap: 8px; }
  .chg { width: 6px; height: 6px; border-radius: 3px; flex: none; }
  .chg.on { background: var(--caution); }
  .cell { display: flex; align-items: center; gap: 8px; min-width: 0; }
  .val { width: 96px; flex: none; text-align: right; font-variant-numeric: tabular-nums; color: var(--text); }
  .na { color: var(--text-3); font-size: 12px; }
  .foot { margin: 14px 2px 4px; }

  .info { display: flex; gap: 16px; margin: 0 28px; padding: 10px 12px 12px 16px; min-height: 96px; }
  .info-text { flex: 1; min-width: 0; }
  .info-head { display: flex; align-items: baseline; gap: 12px; flex-wrap: wrap; }
  .copy { display: flex; flex-direction: column; gap: 2px; flex: none; align-items: stretch; }
  .help { margin: 4px 0 0; color: var(--text-2); }
  .btn.small { min-height: 26px; padding: 2px 8px; font-size: 12px; color: var(--text-2); }

  .cmdbar { display: flex; align-items: center; gap: 8px; padding: 12px 28px 16px; }
  .pending { color: var(--caution); margin-right: 4px; }
  .presets { position: relative; }
  .chev { width: 12px !important; height: 12px !important; color: var(--text-2); }
  .backdrop { position: fixed; inset: 0; z-index: 50; }
  .menu {
    position: absolute; bottom: 40px; left: 0; z-index: 51; width: 380px; padding: 4px 0;
    background: var(--flyout); border: 1px solid var(--control-stroke); border-radius: var(--radius-l); box-shadow: var(--shadow-flyout);
    animation: rise 0.16s cubic-bezier(0.2, 0, 0, 1);
  }
  @keyframes rise { from { opacity: 0; transform: translateY(8px); } }
  .mitem { display: flex; gap: 12px; align-items: center; width: calc(100% - 8px); margin: 2px 4px; padding: 6px 10px; border: 0; border-radius: var(--radius); background: transparent; text-align: left; cursor: default; }
  .mitem:hover { background: var(--subtle-hover); }
  .num { width: 22px; height: 22px; border-radius: 11px; display: grid; place-items: center; font-size: 12px; background: var(--subtle-hover); color: var(--text-2); flex: none; }
  .mtext { display: flex; flex-direction: column; }
  .mnote { padding: 6px 14px 6px; border-top: 1px solid var(--divider); margin-top: 4px; }

  .overlay { position: fixed; inset: 0; z-index: 200; display: grid; place-items: center; background: rgba(0, 0, 0, 0.3); animation: fade 0.12s; }
  @keyframes fade { from { opacity: 0; } }
  .dialog { width: 440px; border-radius: var(--radius-l); background: var(--flyout); border: 1px solid var(--control-stroke); box-shadow: 0 32px 64px rgba(0, 0, 0, 0.28); overflow: hidden; }
  .dbody { padding: 24px; }
  .dbody p { margin: 12px 0 0; }
  .dfoot { display: grid; grid-template-columns: 1fr 1fr 1fr; gap: 8px; padding: 24px; background: var(--bg-solid); border-top: 1px solid var(--divider); }
</style>
