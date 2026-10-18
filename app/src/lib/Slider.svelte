<script lang="ts">
  // Ползунок Fluent. Клик по шкале попадает ровно в то деление, что нарисовано:
  // позиция → значение и значение → позиция считаются одной и той же функцией.
  let {
    value,
    min,
    max,
    step,
    fine,
    big,
    zero = false,
    marker = null,
    disabled = false,
    label = "",
    onchange,
    onfocus,
  }: {
    value: number | null;
    min: number;
    max: number;
    step: number;
    fine: number;
    big: number;
    zero?: boolean;
    marker?: number | null;
    disabled?: boolean;
    label?: string;
    onchange: (v: number) => void;
    onfocus?: () => void;
  } = $props();

  let track: HTMLDivElement;
  let dragging = $state(false);

  const frac = (v: number | null) => {
    if (v == null) return 0;
    if (zero && v === 0) return 1;
    return Math.max(0, Math.min(1, (v - min) / (max - min)));
  };

  const ticks = $derived((max - min) / step <= 20 ? Array.from({ length: (max - min) / step + 1 }, (_, i) => i / ((max - min) / step)) : []);

  function fromX(clientX: number): number {
    const r = track.getBoundingClientRect();
    const fr = Math.max(0, Math.min(1, (clientX - r.left) / r.width));
    if (zero && fr >= 0.999) return 0;
    let v = min + fr * (max - min);
    v = Math.round(v / step) * step;
    return Math.max(min, Math.min(max, v));
  }

  function set(v: number) {
    if (v !== value) onchange(v);
  }

  function down(e: PointerEvent) {
    if (disabled || e.button !== 0) return;
    (e.currentTarget as HTMLElement).setPointerCapture(e.pointerId);
    (e.currentTarget as HTMLElement).focus();
    dragging = true;
    set(fromX(e.clientX));
  }
  function move(e: PointerEvent) {
    if (dragging) set(fromX(e.clientX));
  }
  function up() {
    dragging = false;
  }

  // Клавиатура как в v1: ←→ шаг (к сетке), Shift — точно, PgUp/PgDn — крупно
  function key(e: KeyboardEvent) {
    if (disabled || value == null) return;
    let dir = 0;
    let size = e.shiftKey ? fine : step;
    if (e.key === "ArrowRight" || e.key === "ArrowUp") dir = 1;
    else if (e.key === "ArrowLeft" || e.key === "ArrowDown") dir = -1;
    else if (e.key === "PageUp") { dir = 1; size = big; }
    else if (e.key === "PageDown") { dir = -1; size = big; }
    else if (e.key === "Home") { e.preventDefault(); return set(min); }
    else if (e.key === "End") { e.preventDefault(); return set(zero ? 0 : max); }
    else return;
    e.preventDefault();
    let v = value;
    if (zero && v === 0) {
      // «без лимита» стоит у правого края
      if (dir > 0) return;
      v = max + size;
    }
    let n = v + dir * size;
    if (size !== fine) n = Math.round(n / size) * size;
    if (zero && n > max) n = 0;
    else n = Math.max(min, Math.min(max, n));
    set(n);
  }
</script>

<div
  class="slider"
  class:disabled
  class:dragging
  role="slider"
  tabindex={disabled ? -1 : 0}
  aria-label={label}
  aria-valuemin={min}
  aria-valuemax={max}
  aria-valuenow={value ?? undefined}
  onpointerdown={down}
  onpointermove={move}
  onpointerup={up}
  onpointercancel={up}
  onkeydown={key}
  onfocus={onfocus}
>
  <div class="track" bind:this={track}>
    <div class="rail"></div>
    <div class="fill" style="width: {frac(value) * 100}%"></div>
    {#each ticks as tk}
      <div class="tick" style="left: {tk * 100}%"></div>
    {/each}
    {#if marker != null}
      <div class="marker" style="left: {frac(marker) * 100}%" title="↧"></div>
    {/if}
    <div class="thumb" style="left: {frac(value) * 100}%"></div>
  </div>
</div>

<style>
  .slider { position: relative; flex: 1; min-width: 80px; height: 32px; display: flex; align-items: center; padding: 0 10px; cursor: default; touch-action: none; outline: none; border-radius: var(--radius); }
  .slider:focus-visible { outline: 2px solid var(--text); outline-offset: -2px; }
  .track { position: relative; width: 100%; height: 4px; }
  .rail { position: absolute; inset: 0; border-radius: 2px; background: var(--track); opacity: 0.55; }
  .fill { position: absolute; left: 0; top: 0; bottom: 0; border-radius: 2px; background: var(--accent); }
  .tick { position: absolute; top: 8px; width: 1px; height: 4px; margin-left: -0.5px; background: var(--text-3); opacity: 0.6; }
  .marker {
    position: absolute; top: -8px; width: 2px; height: 20px; margin-left: -1px; border-radius: 1px;
    background: var(--text-2); opacity: 0.85;
  }
  .marker::before {
    content: ""; position: absolute; top: -5px; left: -3px; border: 4px solid transparent; border-top: 5px solid var(--text-2);
  }
  .thumb {
    position: absolute; top: 50%; width: 20px; height: 20px; margin: -10px 0 0 -10px; border-radius: 50%;
    background: var(--flyout); border: 1px solid var(--control-stroke); box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }
  .thumb::after {
    content: ""; position: absolute; inset: 4px; border-radius: 50%; background: var(--accent);
    transition: inset 0.12s cubic-bezier(0.2, 0, 0, 1);
  }
  .slider:hover .thumb::after { inset: 3px; }
  .slider.dragging .thumb::after { inset: 5px; }
  .disabled { pointer-events: none; }
  .disabled .fill, .disabled .thumb::after { background: var(--text-3); }
</style>
