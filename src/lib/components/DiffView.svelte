<script lang="ts">
  import { api } from "../api";
  import type { DiffTarget, RepoStore } from "../repo.svelte";
  import { hunkPatch, parseDiff, type Hunk, type ParsedDiff } from "../diff";
  import { app, confirm, errorText, runOk } from "../state.svelte";
  import { STATUS_LABEL } from "../util";
  import Icon from "./Icon.svelte";

  let { store, target }: { store: RepoStore; target: DiffTarget } = $props();

  const MAX_LINES = 4000;

  let diff = $state<ParsedDiff | null>(null);
  let error = $state("");
  let showAll = $state(false);

  // Reload when the target changes or after any action (e.g. a hunk was staged).
  $effect(() => {
    const t = target;
    void app.refreshTick;
    let cancelled = false;
    api
      .diff(store.path, t.file, t.orig, t.mode, t.hash)
      .then((text) => {
        if (cancelled) return;
        diff = parseDiff(text);
        error = "";
      })
      .catch((e) => !cancelled && (error = errorText(e)));
    return () => (cancelled = true);
  });

  let lineCount = $derived(diff?.hunks.reduce((n, h) => n + h.lines.length, 0) ?? 0);
  let truncated = $derived(!showAll && lineCount > MAX_LINES);
  let visibleHunks = $derived.by(() => {
    if (!diff || !truncated) return diff?.hunks ?? [];
    let budget = MAX_LINES;
    const out: Hunk[] = [];
    for (const h of diff.hunks) {
      if (budget <= 0) break;
      out.push(budget >= h.lines.length ? h : { ...h, lines: h.lines.slice(0, budget) });
      budget -= h.lines.length;
    }
    return out;
  });

  const MODE_LABEL: Record<string, string> = {
    unstaged: "Unstaged changes",
    staged: "Staged changes",
    untracked: "New file",
    commit: "",
  };

  async function apply(h: Hunk, cached: boolean, reverse: boolean) {
    if (!diff) return;
    await runOk(() => api.applyPatch(store.path, hunkPatch(diff!, h), cached, reverse));
  }

  async function discardHunk(h: Hunk) {
    if (!(await confirm("Discard this change?", "The lines in this hunk will be reverted in your working copy. This can't be undone.", "Discard", true))) return;
    await apply(h, false, true);
  }

  let fileStatus = $derived(
    store.status?.files.find((f) => f.path === target.file),
  );
</script>

<div class="diffview">
  <header>
    <button class="btn ghost icon" title="Back to history (Esc)" onclick={() => (store.diff = null)}>
      <Icon name="x" size={16} />
    </button>
    <div class="title">
      <strong class="ellipsis mono">{target.file}</strong>
      {#if target.orig}<span class="muted ellipsis mono">renamed from {target.orig}</span>{/if}
    </div>
    <span class="mode">{MODE_LABEL[target.mode]}</span>
    {#if target.mode === "untracked" || (target.mode === "unstaged" && fileStatus?.conflicted)}
      <button class="btn small primary" onclick={() => runOk(() => api.stage(store.path, [target.file]))}>
        {fileStatus?.conflicted ? "Mark resolved" : "Stage file"}
      </button>
    {:else if target.mode === "unstaged"}
      <button class="btn small primary" onclick={() => runOk(() => api.stage(store.path, [target.file]))}>Stage file</button>
    {:else if target.mode === "staged"}
      <button class="btn small" onclick={() => runOk(() => api.unstage(store.path, [target.file]))}>Unstage file</button>
    {/if}
  </header>

  <div class="content mono">
    {#if error}
      <div class="empty">{error}</div>
    {:else if !diff}
      <div class="empty">Loading…</div>
    {:else if diff.binary}
      <div class="empty">Binary file — no text diff to show.</div>
    {:else if !diff.hunks.length}
      <div class="empty">
        {fileStatus && target.mode !== "commit" ? `${STATUS_LABEL[fileStatus.worktree] ?? "No"} changes to show` : "No changes to show"}
        {#if diff.header.some((l) => l.startsWith("old mode"))}(file mode changed){/if}
      </div>
    {:else}
      {#each visibleHunks as h, hi (hi)}
        <div class="hunk-head">
          <span class="muted">{h.header}</span>
          <span class="hunk-actions">
            {#if target.mode === "unstaged"}
              <button class="btn small ghost" onclick={() => discardHunk(h)}><Icon name="undo" size={12} /> Discard hunk</button>
              <button class="btn small" onclick={() => apply(h, true, false)}><Icon name="plus" size={12} /> Stage hunk</button>
            {:else if target.mode === "staged"}
              <button class="btn small" onclick={() => apply(h, true, true)}><Icon name="minus" size={12} /> Unstage hunk</button>
            {/if}
          </span>
        </div>
        <table>
          <tbody>
            {#each h.lines as l, li (li)}
              <tr class={l.kind}>
                <td class="ln">{l.oldNo ?? ""}</td>
                <td class="ln">{l.newNo ?? ""}</td>
                <td class="sign">{l.kind === "add" ? "+" : l.kind === "del" ? "−" : ""}</td>
                <td class="code selectable">{l.text}</td>
              </tr>
            {/each}
          </tbody>
        </table>
      {/each}
      {#if truncated}
        <div class="empty">
          Showing the first {MAX_LINES.toLocaleString()} of {lineCount.toLocaleString()} lines.
          <button class="btn small" onclick={() => (showAll = true)}>Show everything</button>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .diffview {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 12px 6px 6px;
    border-bottom: 1px solid var(--border);
    background: var(--panel);
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title span {
    font-size: 11px;
  }
  .mode {
    color: var(--muted);
    font-size: 12px;
  }
  .content {
    flex: 1;
    overflow: auto;
    padding-bottom: 24px;
  }
  .hunk-head {
    position: sticky;
    left: 0;
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 10px;
    padding: 4px 10px;
    margin-top: 10px;
    background: var(--panel-2);
    border-top: 1px solid var(--border);
    border-bottom: 1px solid var(--border);
    font-size: 11.5px;
  }
  .hunk-actions {
    display: flex;
    gap: 4px;
    font-family: Inter, system-ui, sans-serif;
  }
  table {
    border-collapse: collapse;
    width: 100%;
    line-height: 20px;
  }
  td {
    padding: 0;
    vertical-align: top;
  }
  .ln {
    width: 1%;
    min-width: 44px;
    padding: 0 8px;
    text-align: right;
    color: var(--faint);
    user-select: none;
    white-space: nowrap;
  }
  .sign {
    width: 16px;
    text-align: center;
    color: var(--faint);
    user-select: none;
  }
  .code {
    white-space: pre;
    padding-right: 16px;
    tab-size: 4;
  }
  tr.add {
    background: var(--add-bg);
  }
  tr.add .code,
  tr.add .sign {
    color: var(--add-fg);
  }
  tr.del {
    background: var(--del-bg);
  }
  tr.del .code,
  tr.del .sign {
    color: var(--del-fg);
  }
  tr.meta .code {
    color: var(--faint);
    font-style: italic;
  }
</style>
