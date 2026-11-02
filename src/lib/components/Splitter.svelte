<script lang="ts">
  // A drag handle on the edge of a column. Drag to resize, arrow keys nudge,
  // double-click restores the default width.
  let {
    width = $bindable(),
    min,
    max,
    initial,
    side,
    label,
  }: {
    width: number;
    min: number;
    max: number;
    initial: number;
    /** Which edge of its column the handle sits on; dragging away from the column widens it. */
    side: "left" | "right";
    label: string;
  } = $props();

  let dragging = $state(false);

  const clamp = (w: number) => Math.round(Math.min(Math.max(w, min), Math.max(min, max)));

  function down(e: PointerEvent) {
    if (e.button !== 0) return;
    e.preventDefault();
    const el = e.currentTarget as HTMLElement;
    el.setPointerCapture(e.pointerId);
    const startX = e.clientX;
    const startW = width;
    dragging = true;
    const move = (ev: PointerEvent) => {
      const dx = ev.clientX - startX;
      width = clamp(startW + (side === "right" ? dx : -dx));
    };
    const up = () => {
      dragging = false;
      el.removeEventListener("pointermove", move);
      el.removeEventListener("pointerup", up);
      el.removeEventListener("pointercancel", up);
    };
    el.addEventListener("pointermove", move);
    el.addEventListener("pointerup", up);
    el.addEventListener("pointercancel", up);
  }

  function key(e: KeyboardEvent) {
    const step = e.shiftKey ? 48 : 16;
    const grow = side === "right" ? "ArrowRight" : "ArrowLeft";
    const shrink = side === "right" ? "ArrowLeft" : "ArrowRight";
    if (e.key === grow) width = clamp(width + step);
    else if (e.key === shrink) width = clamp(width - step);
    else if (e.key === "Home") width = clamp(min);
    else if (e.key === "End") width = clamp(max);
    else return;
    e.preventDefault();
  }
</script>

<!-- A focusable separator is the ARIA pattern for a resize handle. -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div
  class="splitter {side}"
  class:dragging
  role="separator"
  aria-orientation="vertical"
  aria-label={label}
  aria-valuenow={width}
  aria-valuemin={min}
  aria-valuemax={max}
  tabindex="0"
  title="Drag to resize, double-click to reset"
  onpointerdown={down}
  ondblclick={() => (width = clamp(initial))}
  onkeydown={key}
></div>

{#if dragging}
  <!-- Keeps the resize cursor and stops text selection while the pointer is outside the handle. -->
  <div class="shield"></div>
{/if}

<style>
  .splitter {
    position: absolute;
    top: 0;
    bottom: 0;
    width: 7px;
    z-index: 5;
    cursor: col-resize;
    touch-action: none;
  }
  .splitter.right {
    right: -4px;
  }
  .splitter.left {
    left: -4px;
  }
  .splitter::after {
    content: "";
    position: absolute;
    top: 0;
    bottom: 0;
    left: 2px;
    width: 3px;
    border-radius: var(--radius-full);
    background: transparent;
    transition: background-color 120ms ease;
  }
  .splitter:hover::after,
  .splitter:focus-visible::after,
  .splitter.dragging::after {
    background: var(--accent);
    transition-delay: 80ms;
  }
  .splitter:focus-visible {
    outline: none;
  }
  .shield {
    position: fixed;
    inset: 0;
    z-index: var(--layer-menu);
    cursor: col-resize;
  }
</style>
