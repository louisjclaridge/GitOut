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
  <span class="st st-{status === '?' ? 'N' : status}" title={STATUS_LABEL[status] ?? status}>{status === "?" ? "+" : status}</span>
  <span class="name">{name}</span>
  <span class="dir ellipsis">{dir}</span>
  {#if actions}<span class="actions">{@render actions()}</span>{/if}
</div>

<style>
  .file {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 28px;
    padding: 0 6px 0 12px;
    min-width: 0;
    cursor: pointer;
  }
  .file:hover {
    background: var(--hover);
  }
  .file.selected {
    background: var(--selected);
  }
  .st {
    width: 16px;
    height: 16px;
    border-radius: 4px;
    font-size: 10px;
    font-weight: 700;
    display: grid;
    place-items: center;
    flex-shrink: 0;
    color: #fff;
    background: var(--warn);
  }
  .st-A,
  .st-N {
    background: var(--ok);
  }
  .st-D {
    background: var(--danger);
  }
  .st-R,
  .st-C {
    background: var(--lane-4);
  }
  .st-U {
    background: var(--danger);
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
    color: var(--faint);
    font-size: 12px;
    direction: rtl;
    text-align: left;
  }
  .actions {
    display: none;
    gap: 2px;
  }
  .file:hover .actions {
    display: flex;
  }
</style>
