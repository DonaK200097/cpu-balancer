// Общее состояние интерфейса (данные приходят из Rust).
import type { Control, Meta } from "./api";

export const ui = $state({
  meta: null as Meta | null,
  page: "editor" as "editor" | "eco" | "settings",
  toast: null as { text: string; kind: "ok" | "warn" | "err" } | null,
  update: null as { version: string; url: string } | null,
  startupMs: 0,
});

let toastTimer: ReturnType<typeof setTimeout> | undefined;

export function toast(text: string, kind: "ok" | "warn" | "err" = "ok") {
  ui.toast = { text, kind };
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => (ui.toast = null), kind === "ok" ? 4000 : 8000);
}

export function controlDef(id: string): Control | undefined {
  for (const g of ui.meta?.groups ?? []) for (const c of g.controls) if (c.id === id) return c;
}
