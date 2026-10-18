<script lang="ts">
  import { onMount } from "svelte";
  import { api, checkUpdate, type AutostartStatus, type EcoInfo } from "./api";
  import { i18n, t } from "./i18n.svelte";
  import { ui, toast } from "./store.svelte";
  import Select from "./Select.svelte";

  let autostart = $state<AutostartStatus | null>(null);
  let busy = $state(false);
  let info = $state<EcoInfo | null>(null);
  let langSetting = $state(ui.meta?.lang_setting ?? "");
  let checkUpdates = $state(ui.meta?.check_updates ?? true);
  let checking = $state(false);

  async function setLang(v: string) {
    langSetting = v;
    i18n.lang = await api.setLang(v);
    document.documentElement.lang = i18n.lang;
  }

  async function toggleAutostart() {
    if (!autostart || busy) return;
    busy = true;
    try {
      autostart = await api.autostartSet(!autostart.enabled);
    } catch (e) {
      toast(String(e), "err");
      autostart = await api.autostartRefresh();
    } finally {
      busy = false;
    }
  }

  async function replaceV1() {
    busy = true;
    try {
      await api.autostartRemoveV1();
      autostart = await api.autostartSet(true);
    } catch (e) {
      toast(String(e), "err");
    } finally {
      busy = false;
    }
  }

  async function togglePause() {
    if (!info) return;
    await api.setPaused(!info.paused);
    info = await api.ecoInfo();
  }

  async function toggleUpdates() {
    checkUpdates = !checkUpdates;
    await api.setCheckUpdates(checkUpdates);
  }

  async function checkNow() {
    checking = true;
    const u = await checkUpdate(ui.meta!.version);
    checking = false;
    ui.update = u;
    if (!u) toast(t("settings.upToDate"));
  }

  onMount(async () => {
    info = await api.ecoInfo();
    autostart = info.autostart.known ? info.autostart : await api.autostartRefresh();
  });
</script>

<div class="page">
  <header><h1 class="title">{t("nav.settings")}</h1></header>
  <div class="scroll">
    <div class="card item">
      <div class="itext"><span>{t("settings.lang")}</span></div>
      <Select
        width="200px"
        options={[
          { value: "", label: t("settings.langSystem") },
          { value: "ru", label: "Русский" },
          { value: "en", label: "English" },
        ]}
        value={langSetting}
        onchange={(v) => setLang(String(v))}
      />
    </div>

    <div class="card item">
      <div class="itext">
        <span>{t("settings.autostart")}</span>
        <span class="caption">{t("settings.autostartDesc")}</span>
      </div>
      {#if autostart}
        <button class="toggle" class:on={autostart.enabled} class:disabled={busy} onclick={toggleAutostart} role="switch" aria-checked={autostart.enabled}>
          <span class="tlabel">{autostart.enabled ? t("common.on") : t("common.off")}</span><span class="track"></span>
        </button>
      {:else}
        <span class="caption">{t("settings.checking")}</span>
      {/if}
    </div>
    {#if autostart?.v1_task}
      <div class="card item warnbox">
        <div class="itext"><span>{t("settings.v1")}</span></div>
        <button class="btn accent" onclick={replaceV1} disabled={busy}>{t("settings.v1replace")}</button>
      </div>
    {/if}

    <div class="card item">
      <div class="itext"><span>{t("settings.pause")}</span></div>
      {#if info}
        <button class="toggle" class:on={info.paused} onclick={togglePause} role="switch" aria-checked={info.paused}>
          <span class="tlabel">{info.paused ? t("common.on") : t("common.off")}</span><span class="track"></span>
        </button>
      {/if}
    </div>

    <div class="card item">
      <div class="itext">
        <span>{t("settings.updates")}</span>
        <span class="caption">{t("settings.updatesDesc")}</span>
      </div>
      <button class="btn" onclick={checkNow} disabled={checking}>{checking ? t("settings.checking") : t("settings.checkNow")}</button>
      <button class="toggle" class:on={checkUpdates} onclick={toggleUpdates} role="switch" aria-checked={checkUpdates}>
        <span class="tlabel">{checkUpdates ? t("common.on") : t("common.off")}</span><span class="track"></span>
      </button>
    </div>

    <div class="card item">
      <div class="itext">
        <span>{t("settings.dataDir")}</span>
        <span class="caption">{ui.meta?.data_dir} — {t("settings.dataDirDesc")}</span>
      </div>
      <button class="btn" onclick={api.openDataDir}>{t("settings.open")}</button>
    </div>

    <h2 class="group">{t("settings.about")}</h2>
    <div class="card item about">
      <img src="/icon.png" alt="" width="40" height="40" />
      <div class="itext">
        <span class="body-strong">CPU Balancer {ui.meta?.version}</span>
        <span class="caption">{t("app.subtitle")} · {t("settings.startup", { ms: ui.startupMs })}</span>
        <span class="caption">{t("settings.tray")}</span>
      </div>
      <button class="btn" onclick={() => api.openUrl("https://github.com/DonaK200097/cpu-balancer")}>GitHub</button>
      <button class="btn" onclick={() => api.openUrl("https://github.com/DonaK200097/cpu-balancer/tree/main/legacy/powershell")}>{t("settings.v1link")}</button>
    </div>
    <div class="quit"><button class="btn" onclick={api.quit}>{t("settings.quit")}</button></div>
  </div>
</div>

<style>
  .page { display: flex; flex-direction: column; height: 100%; min-height: 0; }
  header { padding: 20px 28px 12px; }
  .scroll { flex: 1; min-height: 0; overflow-y: auto; padding: 0 28px 20px; display: flex; flex-direction: column; gap: 4px; }
  .item { display: flex; align-items: center; gap: 12px; min-height: 68px; padding: 12px 16px; }
  .itext { flex: 1; display: flex; flex-direction: column; min-width: 0; }
  .warnbox { border-color: color-mix(in srgb, var(--caution) 50%, transparent); }
  .group { font-size: 14px; margin: 16px 0 4px 2px; }
  .tlabel { min-width: 34px; text-align: right; }
  .toggle { background: none; border: 0; padding: 0; }
  .about img { border-radius: 8px; }
  .quit { margin-top: 12px; }
</style>
