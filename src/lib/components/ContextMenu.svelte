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
  <div class="k-menu" bind:this={el} style="left:{pos.x}px; top:{pos.y}px" role="menu">
    {#each menu.items as item, i (i)}
      {#if item === "sep"}
        <div class="k-menu-sep" role="separator"></div>
      {:else}
        <button
          class="k-menu-item"
          class:danger={item.danger}
          disabled={item.disabled}
          role="menuitem"
          onclick={() => {
            closeMenu();
            item.action();
          }}
        >
          <span class="ic">{#if item.icon}<Icon name={item.icon} size={14} />{/if}</span>
          <span class="ellipsis">{item.label}</span>
        </button>
      {/if}
    {/each}
  </div>
{/if}

<style>
  .k-menu {
    position: fixed;
  }
  .ic {
    display: inline-flex;
    width: 14px;
    flex: none;
  }
</style>
