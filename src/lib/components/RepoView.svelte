<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { app, confirm, DETAILS_WIDTH, layout, run, runOk, saveLayout, SIDEBAR_WIDTH } from "../state.svelte";
  import { RepoStore } from "../repo.svelte";
  import Toolbar from "./Toolbar.svelte";
  import Sidebar from "./Sidebar.svelte";
  import Graph from "./Graph.svelte";
  import WipPanel from "./WipPanel.svelte";
  import CommitPanel from "./CommitPanel.svelte";
  import DiffView from "./DiffView.svelte";
  import MergeView from "./MergeView.svelte";
  import ReleaseView from "./ReleaseView.svelte";
  import ReleaseEditor from "./ReleaseEditor.svelte";
  import RunView from "./RunView.svelte";
  import Splitter from "./Splitter.svelte";
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
      timer = setTimeout(() => {
        void store.load();
        if (store.hasCi) void store.loadRuns();
      }, 150);
    };
    window.addEventListener("focus", onFocus);

    // CI/CD runs: load once, then poll — quickly while something is running.
    // The run view polls its own run faster.
    void store.loadRuns();
    let poll: ReturnType<typeof setTimeout>;
    const again = () => {
      poll = setTimeout(async () => {
        if (store.hasCi && document.visibilityState === "visible" && store.center.kind !== "run") await store.loadRuns();
        again();
      }, store.runsActive ? 10_000 : 60_000);
    };
    again();

    return () => {
      window.removeEventListener("focus", onFocus);
      clearTimeout(timer);
      clearTimeout(poll);
    };
  });

  $effect(() => {
    void layout.sidebar;
    void layout.details;
    saveLayout();
  });

  // Keep at least this much for the history between the two side columns.
  const CENTER_MIN = 360;
  let columnsWidth = $state(0);

  const STATE_LABEL: Record<string, string> = {
    merging: "A merge is in progress",
    rebasing: "A rebase is in progress",
    "cherry-picking": "A cherry-pick is in progress",
    reverting: "A revert is in progress",
  };

  let conflicts = $derived(store.conflicted.length);
  let center = $derived(store.center);
  let merging = $derived(center.kind === "merge");

  async function abort() {
    if (!(await confirm("Abort the operation?", "This undoes the operation and returns to where you were before it started.", "Abort", true))) return;
    if (await runOk(() => api.operationAbort(repo), { done: "Aborted" })) {
      if (store.center.kind === "merge") store.center = { kind: "graph" };
    }
  }

  function onKey(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement || e.target instanceof HTMLTextAreaElement) return;
    // The merge editor and release editor hold unsaved work, so Escape doesn't close them.
    if (e.key === "Escape" && (center.kind === "diff" || center.kind === "release" || center.kind === "run")) store.center = { kind: "graph" };
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "r") {
      e.preventDefault();
      void store.load();
    }
  }
</script>

<svelte:window onkeydown={onKey} />

<div class="repo">
  <Toolbar {store} />

  {#if store.status?.state || conflicts}
    <div class="notice">
      <div class="k-banner warning">
        <Icon name="warning" size={20} />
        <div class="body">
          {#if store.status?.state}
            <span class="title">{STATE_LABEL[store.status.state]}.</span>
            {#if conflicts}
              Resolve {conflicts} conflicted file{conflicts === 1 ? "" : "s"}, then continue.
            {:else}
              All conflicts are resolved. You can continue.
            {/if}
          {:else}
            <span class="title">{conflicts} file{conflicts === 1 ? " has" : "s have"} conflicts.</span>
            Resolve them before you commit.
          {/if}
        </div>
        {#if conflicts && !merging}
          <button class="k-btn tinted" onclick={() => store.openMerge(store.conflicted[0].path)}><Icon name="merge" /> Resolve conflicts</button>
        {/if}
        {#if store.status?.state}
          <button class="k-btn ghost" onclick={abort}>Abort…</button>
          <button
            class="k-btn primary"
            disabled={conflicts > 0}
            onclick={() => run(() => api.operationContinue(repo), { done: "Continued" })}>Continue</button
          >
        {/if}
      </div>
    </div>
  {/if}

  <div
    class="columns"
    class:merging
    bind:clientWidth={columnsWidth}
    style="--sidebar-w:{layout.sidebar}px; --details-w:{layout.details}px"
  >
    <!-- Hidden rather than removed while merging, so it keeps its state. -->
    <div class="side" class:hidden={merging}>
      <Sidebar {store} />
      <Splitter
        bind:width={layout.sidebar}
        min={180}
        max={Math.min(560, columnsWidth - layout.details - CENTER_MIN)}
        initial={SIDEBAR_WIDTH}
        side="right"
        label="Resize sidebar"
      />
    </div>
    <div class="center">
      {#if center.kind === "merge"}
        {#key center.file}<MergeView {store} file={center.file} />{/key}
      {:else if center.kind === "release"}
        <ReleaseView {store} id={center.id} />
      {:else if center.kind === "run"}
        {#key center.id}<RunView {store} id={center.id} />{/key}
      {:else if center.kind === "release-edit"}
        {#key center}<ReleaseEditor {store} id={center.id} tag={center.tag} target={center.target} />{/key}
      {:else if center.kind === "diff"}
        <DiffView {store} target={center.target} />
      {:else}
        <Graph {store} />
      {/if}
    </div>
    <aside class="details">
      <Splitter
        bind:width={layout.details}
        min={260}
        max={Math.min(720, columnsWidth - (merging ? 0 : layout.sidebar) - CENTER_MIN)}
        initial={DETAILS_WIDTH}
        side="left"
        label="Resize details panel"
      />
      {#if store.selected?.kind === "wip"}
        <WipPanel {store} />
      {:else if store.selected?.kind === "commit"}
        {#key store.selected.hash}
          <CommitPanel {store} hash={store.selected.hash} />
        {/key}
      {:else if store.loaded}
        <div class="empty-note">Select a commit to see its details.</div>
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
  .notice {
    padding: 0 var(--space-8) var(--space-8);
    background: var(--bg);
    border-bottom: 1px solid var(--border);
  }
  .notice .k-banner {
    padding: var(--space-6) var(--space-6) var(--space-6) var(--space-12);
  }
  .notice .title {
    margin-right: var(--space-4);
  }
  .columns {
    flex: 1;
    min-height: 0;
    display: grid;
    /* min() keeps the history visible if the window shrinks below the saved widths. */
    grid-template-columns: min(var(--sidebar-w), 30%) minmax(0, 1fr) min(var(--details-w), 40%);
  }
  .side {
    position: relative;
    display: grid;
    /* minmax(0, …) so a long branch name can't widen the column past its set width. */
    grid-template-columns: minmax(0, 1fr);
    grid-template-rows: minmax(0, 1fr);
    min-width: 0;
    min-height: 0;
  }
  .side.hidden {
    display: none;
  }
  /* The merge editor needs the width for its three panes. */
  .columns.merging {
    grid-template-columns: minmax(0, 1fr) min(var(--details-w), 40%);
  }
  .center {
    min-width: 0;
    min-height: 0;
    display: flex;
    flex-direction: column;
    background: var(--surface);
  }
  .details {
    position: relative;
    min-width: 0;
    min-height: 0;
    border-left: 1px solid var(--border);
    background: var(--surface);
    display: flex;
    flex-direction: column;
  }
</style>
