<script lang="ts">
  import { menu, closeMenu } from "../menu.svelte";
  import Icon from "./Icon.svelte";

  let el: HTMLDivElement | undefined = $state();
  let pos = $state({ x: 0, y: 0 });

  // Keep the menu on-screen when opened near the window edge.
  $effect(() => {
    if (!menu.open || !el) return;
    const r = el.getBoundingClientRect();
    pos = {
      x: Math.min(menu.x, window.innerWidth - r.width - 6),
      y: Math.min(menu.y, window.innerHeight - r.height - 6),
    };
  });
</script>

<svelte:window
  onmousedown={(e) => {
    if (menu.open && el && !el.contains(e.target as Node)) closeMenu();
  }}
  onkeydown={(e) => e.key === "Escape" && closeMenu()}
  onblur={closeMenu}
  onresize={closeMenu}
/>

{#if menu.open}
  <div class="menu" bind:this={el} style="left:{pos.x}px; top:{pos.y}px" role="menu">
    {#each menu.items as item, i (i)}
      {#if item === "sep"}
        <div class="sep"></div>
      {:else}
        <button
          class="item"
          class:danger={item.danger}
          disabled={item.disabled}
          role="menuitem"
          onclick={() => {
            closeMenu();
            item.action();
          }}
        >
          <span class="ic">{#if item.icon}<Icon name={item.icon} size={14} />{/if}</span>
          {item.label}
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .menu {
    position: fixed;
    z-index: 1000;
    min-width: 200px;
    max-width: 340px;
    padding: 4px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 8px;
    box-shadow: var(--shadow);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 8px;
    width: 100%;
    padding: 6px 10px;
    border: 0;
    border-radius: 5px;
    background: none;
    text-align: left;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .item:hover:not(:disabled) {
    background: var(--accent);
    color: var(--accent-text);
  }
  .item:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .item.danger {
    color: var(--danger);
  }
  .item.danger:hover {
    background: var(--danger);
    color: #fff;
  }
  .ic {
    width: 14px;
    display: inline-flex;
  }
  .sep {
    height: 1px;
    margin: 4px 6px;
    background: var(--border);
  }
</style>
