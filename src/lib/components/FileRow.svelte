<script lang="ts">
  import type { Snippet } from "svelte";
  import { STATUS_LABEL } from "../util";

  let {
    path,
    status,
    selected = false,
    onclick,
    oncontextmenu,
    actions,
  }: {
    path: string;
    status: string;
    selected?: boolean;
    onclick: () => void;
    oncontextmenu?: (e: MouseEvent) => void;
    actions?: Snippet;
  } = $props();

  let slash = $derived(path.lastIndexOf("/"));
  let name = $derived(path.slice(slash + 1));
  let dir = $derived(slash >= 0 ? path.slice(0, slash) : "");
</script>

<div
  class="file"
  class:selected
  role="button"
  tabindex="0"
  title={path}
  {onclick}
  {oncontextmenu}
  onkeydown={(e) => e.key === "Enter" && onclick()}
>
  <span class="st st-{status === '?' ? 'N' : status}" title={STATUS_LABEL[status] ?? status} aria-label={STATUS_LABEL[status] ?? status}>{status === "?" ? "+" : status}</span>
  <span class="name">{name}</span>
  <span class="dir ellipsis">{dir}</span>
  {#if actions}<span class="actions">{@render actions()}</span>{/if}
</div>

<style>
  .file {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    height: var(--control-md);
    margin: 0 var(--space-8);
    padding: 0 var(--space-2) 0 var(--space-8);
    border-radius: var(--radius-sm);
    min-width: 0;
    cursor: pointer;
  }
  .file:hover {
    background: var(--surface-hover);
  }
  .file.selected {
    background: var(--accent-soft);
  }
  /* Status letter as a toned chip: word in the tooltip, letter and colour together. */
  .st {
    width: 18px;
    height: 18px;
    border-radius: var(--radius-xs);
    font-family: var(--font-mono);
    font-size: 11px;
    font-weight: 600;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    background: var(--warning-soft);
    color: var(--warning);
  }
  .st-A,
  .st-N {
    background: var(--success-soft);
    color: var(--success);
  }
  .st-D,
  .st-U {
    background: var(--danger-soft);
    color: var(--danger);
  }
  .st-R,
  .st-C {
    background: var(--accent-soft);
    color: var(--accent-text);
  }
  .name {
    white-space: nowrap;
    flex-shrink: 0;
    max-width: 70%;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .dir {
    flex: 1;
    color: var(--text-3);
    font-size: 12px;
    direction: rtl;
    text-align: left;
  }
  .actions {
    display: none;
    gap: var(--space-2);
  }
  .file:hover .actions,
  .file:focus-within .actions {
    display: flex;
  }
</style>
