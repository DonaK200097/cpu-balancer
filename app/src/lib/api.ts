// Типы и вызовы команд Rust. Вся логика — в бэкенде, здесь только транспорт.
import { invoke } from "@tauri-apps/api/core";

export type ModeOpt = { id: string; map: [string, number][]; backup: boolean };
export type Control =
  | { kind: "slider"; id: string; keys: string[]; min: number; max: number; step: number; fine: number; big: number; unit: string; zero: boolean }
  | { kind: "enum"; id: string; keys: string[]; opts: number[] }
  | { kind: "mode"; id: string; opts: ModeOpt[] };
export type Group = { id: string; controls: Control[] };
export type Preset = { id: string };

export type Meta = {
  version: string;
  lang: "ru" | "en";
  lang_setting: string;
  accent: string | null;
  mica: boolean;
  groups: Group[];
  presets: Preset[];
  emask: string;
  ecores: number;
  ncpu: number;
  data_dir: string;
  check_updates: boolean;
};

export type Scheme = { guid: string; name: string; active: boolean };
export type Cell = { value: number | null; mode: number };
export type CtlView = { id: string; available: boolean; changed: boolean; cur: [Cell, Cell]; backup: [Cell, Cell] };
export type EditorView = { scheme: string; name: string; active: boolean; pending: number; controls: CtlView[] };
export type SaveResult = { saved: number; failed: string[]; active: boolean };

export type EcoRow = { name: string; mode: number; procs: number; cpu: number; eco: number };
export type AutostartStatus = { known: boolean; enabled: boolean; v1_task: boolean };
export type EcoInfo = { method: string; paused: boolean; applied: number; autostart: AutostartStatus };

export const api = {
  meta: () => invoke<Meta>("get_meta"),
  uiReady: () => invoke<number>("ui_ready"),
  schemes: () => invoke<Scheme[]>("list_schemes"),
  setActive: (guid: string) => invoke<Scheme[]>("set_active", { guid }),
  openScheme: (guid: string) => invoke<EditorView>("open_scheme", { guid }),
  reloadScheme: () => invoke<EditorView>("reload_scheme"),
  setControl: (id: string, col: number, value: number) => invoke<EditorView>("set_control", { id, col, value }),
  copyControl: (id: string, col: number) => invoke<EditorView>("copy_control", { id, col }),
  applyPreset: (id: string) => invoke<EditorView>("apply_preset", { id }),
  discard: () => invoke<EditorView>("discard"),
  restoreBackup: () => invoke<EditorView>("restore_backup"),
  save: () => invoke<{ result: SaveResult; view: EditorView }>("save"),
  powerStatus: () => invoke<boolean>("power_status"),
  ecoRows: () => invoke<EcoRow[]>("eco_rows"),
  ecoSet: (name: string, mode: number) => invoke<{ ok: number; bad: number; saved: boolean }>("eco_set", { name, mode }),
  ecoInfo: () => invoke<EcoInfo>("eco_info"),
  setPaused: (paused: boolean) => invoke<void>("set_paused", { paused }),
  autostartRefresh: () => invoke<AutostartStatus>("autostart_refresh"),
  autostartSet: (enabled: boolean) => invoke<AutostartStatus>("autostart_set", { enabled }),
  autostartRemoveV1: () => invoke<AutostartStatus>("autostart_remove_v1"),
  setLang: (lang: string) => invoke<"ru" | "en">("set_lang", { lang }),
  setCheckUpdates: (enabled: boolean) => invoke<void>("set_check_updates", { enabled }),
  openUrl: (url: string) => invoke<void>("open_url", { url }),
  openDataDir: () => invoke<void>("open_data_dir"),
  quit: () => invoke<void>("quit"),
};

const RELEASES = "https://api.github.com/repos/DonaK200097/cpu-balancer/releases/latest";

/// Проверка обновлений: только уведомление. null — нет новой версии или нет сети.
export async function checkUpdate(current: string): Promise<{ version: string; url: string } | null> {
  try {
    const r = await fetch(RELEASES, { headers: { Accept: "application/vnd.github+json" } });
    if (!r.ok) return null;
    const j = await r.json();
    const tag = String(j.tag_name ?? "").replace(/^v/, "");
    const cmp = (a: string, b: string) => {
      const pa = a.split(".").map((x) => parseInt(x) || 0);
      const pb = b.split(".").map((x) => parseInt(x) || 0);
      for (let i = 0; i < 3; i++) if ((pa[i] ?? 0) !== (pb[i] ?? 0)) return (pa[i] ?? 0) - (pb[i] ?? 0);
      return 0;
    };
    if (tag && cmp(tag, current) > 0) return { version: tag, url: String(j.html_url) };
  } catch {}
  return null;
}
