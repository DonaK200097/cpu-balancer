<script lang="ts">
  // Выпадающий список в стиле ComboBox Windows 11.
  type Opt = { value: string | number; label: string; hint?: string };
  let {
    options,
    value,
    display = "",
    disabled = false,
    width = "",
    onchange,
    onfocus,
  }: {
    options: Opt[];
    value: string | number | null;
    display?: string;
    disabled?: boolean;
    width?: string;
    onchange: (v: string | number) => void;
    onfocus?: () => void;
  } = $props();

  let open = $state(false);
  let hi = $state(0);
  let btn: HTMLButtonElement;
  let pos = $state({ left: 0, top: 0, width: 0, up: false });

  const current = $derived(options.find((o) => o.value === value));

  function toggle() {
    if (disabled) return;
    if (open) return close();
    const r = btn.getBoundingClientRect();
    const h = Math.min(options.length * 36 + 8, 360);
    const up = r.bottom + h > window.innerHeight - 8 && r.top - h > 8;
    pos = { left: r.left, top: up ? r.top - h - 4 : r.bottom + 4, width: Math.max(r.width, 160), up };
    hi = Math.max(0, options.findIndex((o) => o.value === value));
    open = true;
    setTimeout(() => window.addEventListener("pointerdown", outside, { once: true }), 0);
  }
  function close() {
    open = false;
  }
  function outside(e: PointerEvent) {
    // клик по самой кнопке закроет список через toggle — иначе он закроется и тут же откроется снова
    if (btn.contains(e.target as Node)) return;
    if (!(e.target as HTMLElement).closest?.(".flyout")) close();
    else window.addEventListener("pointerdown", outside, { once: true });
  }
  function pick(o: Opt) {
    close();
    if (o.value !== value) onchange(o.value);
    btn.focus();
  }
  function key(e: KeyboardEvent) {
    if (disabled) return;
    if (!open) {
      if (e.key === "Enter" || e.key === " " || e.key === "F4") { e.preventDefault(); toggle(); }
      else if (e.key === "ArrowDown" || e.key === "ArrowUp") {
        // как в v1: стрелки листают варианты без раскрытия
        e.preventDefault();
        const i = options.findIndex((o) => o.value === value);
        const n = Math.max(0, Math.min(options.length - 1, (i < 0 ? 0 : i) + (e.key === "ArrowDown" ? 1 : -1)));
        if (options[n] && options[n].value !== value) onchange(options[n].value);
      }
      return;
    }
    if (e.key === "Escape") { e.preventDefault(); close(); }
    else if (e.key === "ArrowDown") { e.preventDefault(); hi = Math.min(options.length - 1, hi + 1); }
    else if (e.key === "ArrowUp") { e.preventDefault(); hi = Math.max(0, hi - 1); }
    else if (e.key === "Enter" || e.key === " ") { e.preventDefault(); pick(options[hi]); }
    else if (e.key === "Tab") close();
  }
</script>

<button class="combo" class:open {disabled} bind:this={btn} onclick={toggle} onkeydown={key} onfocus={onfocus} style:width type="button">
  <span class="label">{current ? current.label : display}</span>
  <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 6 8 10.5 12.5 6" fill="none" stroke="currentColor" stroke-width="1.2" stroke-linecap="round" stroke-linejoin="round" /></svg>
</button>

{#if open}
  <div class="flyout" class:up={pos.up} role="listbox" style="left: {pos.left}px; top: {pos.top}px; min-width: {pos.width}px">
    {#each options as o, i}
      <button
        class="item"
        class:sel={o.value === value}
        class:hi={i === hi}
        role="option"
        aria-selected={o.value === value}
        type="button"
        onpointerenter={() => (hi = i)}
        onclick={() => pick(o)}
      >
        <span>{o.label}</span>
        {#if o.hint}<span class="hint">{o.hint}</span>{/if}
      </button>
    {/each}
  </div>
{/if}

<style>
  .combo {
    display: flex; align-items: center; gap: 8px; height: 32px; padding: 0 10px 0 11px; min-width: 120px; max-width: 100%;
    border-radius: var(--radius); background: var(--control); border: 1px solid var(--control-stroke);
    border-bottom-color: var(--control-stroke-bottom); text-align: left; cursor: default;
  }
  .combo:hover:not(:disabled) { background: var(--control-hover); }
  .combo:disabled { color: var(--text-3); }
  .combo .label { flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .combo svg { width: 12px; height: 12px; flex: none; color: var(--text-2); }
  .flyout {
    position: fixed; z-index: 100; padding: 4px 0; max-height: 360px; overflow-y: auto;
    background: var(--flyout); border: 1px solid var(--control-stroke); border-radius: var(--radius-l);
    box-shadow: var(--shadow-flyout); animation: drop 0.16s cubic-bezier(0.2, 0, 0, 1);
  }
  .flyout.up { animation-name: rise; }
  @keyframes drop { from { opacity: 0; transform: translateY(-8px); } }
  @keyframes rise { from { opacity: 0; transform: translateY(8px); } }
  .item {
    position: relative; display: flex; align-items: baseline; gap: 10px; width: calc(100% - 8px); margin: 2px 4px; height: 32px;
    padding: 0 12px; border: 0; border-radius: var(--radius); background: transparent; text-align: left; cursor: default; white-space: nowrap;
  }
  .item.hi { background: var(--subtle-hover); }
  .item.sel { background: var(--subtle-hover); }
  .item.sel::before {
    content: ""; position: absolute; left: 0; top: 9px; height: 14px; width: 3px; border-radius: 2px; background: var(--accent);
  }
  .item span:first-child { line-height: 32px; }
  .hint { color: var(--text-3); font-size: 12px; }
</style>
