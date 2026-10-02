<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { app, confirm, run } from "../state.svelte";
  import { RepoStore } from "../repo.svelte";
  import Toolbar from "./Toolbar.svelte";
  import Sidebar from "./Sidebar.svelte";
  import Graph from "./Graph.svelte";
  import WipPanel from "./WipPanel.svelte";
  import CommitPanel from "./CommitPanel.svelte";
  import DiffView from "./DiffView.svelte";
  import Icon from "./Icon.svelte";

  let { repo }: { repo: string } = $props();

  // Recreated per tab ({#key} in App), so capturing the initial prop is intended.
  // svelte-ignore state_referenced_locally
  const store = new RepoStore(repo);

  // Reload after any action, and when the window regains focus (files may
  // have been edited in another app).
  $effect(() => {
    void app.refreshTick;
    void store.load();
  });

  onMount(() => {
    let timer: ReturnType<typeof setTimeout>;
    const onFocus = () => {
      clearTimeout(timer);
      timer = setTimeout(() => store.load(), 150);
    };
    window.addEventListener("focus", onFocus);
    return () => window.removeEventListener("focus", onFocus);
  });

  const STATE_LABEL: Record<string, string> = {
    merging: "A merge is in progress",
    rebasing: "A rebase is in progress",
    "cherry-picking": "A cherry-pick is in progress",
    reverting: "A revert is in progress",
  };

  let conflicts = $derived(store.status?.files.filter((f) => f.conflicted).length ?? 0);

  function onKey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    if (e.key === "Escape" && store.diff) store.diff = null;
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r") {
      e.preventDefault();
      void store.load();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="repo">
  <Toolbar {store} />

  {#if store.status?.state}
    <div class="banner">
      <Icon name="warning" size={15} />
      <span>
        <strong>{STATE_LABEL[store.status.state]}.</strong>
        {#if conflicts}
          Resolve {conflicts} conflicted file{conflicts === 1 ? "" : "s"}, stage them, then continue.
        {:else}
          All conflicts resolved. You can continue.
        {/if}
      </span>
      <button
        class="btn small primary"
        disabled={conflicts > 0}
        onclick={() => run(() => api.operationContinue(repo), { done: "Continued" })}>Continue</button
      >
      <button
        class="btn small"
        onclick={async () =>
          (await confirm("Abort?", "This undoes the operation and returns to where you were before it started.", "Abort", true)) &&
          run(() => api.operationAbort(repo), { done: "Aborted" })}>Abort</button
      >
    </div>
  {/if}

  <div class="body">
    <Sidebar {store} />
    <div class="center">
      {#if store.diff}
        <DiffView {store} target={store.diff} />
      {:else}
        <Graph {store} />
      {/if}
    </div>
    <aside class="details">
      {#if store.selected?.kind === "wip"}
        <WipPanel {store} />
      {:else if store.selected?.kind === "commit"}
        {#key store.selected.hash}
          <CommitPanel {store} hash={store.selected.hash} />
        {/key}
      {:else if store.loaded}
        <div class="empty">Select a commit to see its details.</div>
      {/if}
    </aside>
  </div>
</div>

<style>
  .repo {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .banner {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 7px 14px;
    background: color-mix(in srgb, var(--warn) 14%, var(--panel));
    border-bottom: 1px solid var(--border);
  }
  .banner :global(svg) {
    color: var(--warn);
  }
  .banner span {
    flex: 1;
  }
  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: 230px minmax(0, 1fr) 360px;
  }
  .center {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
  }
  .details {
    min-height: 0;
    border-left: 1px solid var(--border);
    background: var(--panel);
    display: flex;
    flex-direction: column;
  }
  @media (max-width: 1100px) {
    .body {
      grid-template-columns: 200px minmax(0, 1fr) 300px;
    }
  }
</style>
