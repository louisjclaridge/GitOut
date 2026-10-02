<script lang="ts">
  import { openPath, revealItemInDir } from "@tauri-apps/plugin-opener";
  import { api, type FileChange } from "../api";
  import type { RepoStore } from "../repo.svelte";
  import { showMenu, type MenuItem } from "../menu.svelte";
  import { confirm, run, runOk, toast } from "../state.svelte";
  import FileRow from "./FileRow.svelte";
  import Icon from "./Icon.svelte";

  let { store }: { store: RepoStore } = $props();

  // The panel is recreated per repository, so the initial path is the right one.
  // svelte-ignore state_referenced_locally
  const draftKey = `gitout.draft:${store.path}`;
  const saved = JSON.parse(localStorage.getItem(draftKey) ?? "{}");
  let summary = $state<string>(saved.summary ?? "");
  let body = $state<string>(saved.body ?? "");
  let amend = $state(false);
  let committing = $state(false);

  $effect(() => {
    localStorage.setItem(draftKey, JSON.stringify({ summary, body }));
  });

  let files = $derived(store.status?.files ?? []);
  let conflicted = $derived(files.filter((f) => f.conflicted));
  let unstaged = $derived(files.filter((f) => !f.conflicted && f.worktree !== "."));
  let staged = $derived(files.filter((f) => !f.conflicted && f.index !== "." && f.index !== "?"));

  const join = (a: string, b: string) => `${a.replace(/[\\/]$/, "")}/${b}`;

  function isSel(f: FileChange, mode: string) {
    return store.diff?.file === f.path && store.diff.mode === mode;
  }

  function showDiff(f: FileChange, which: "unstaged" | "staged") {
    const mode = which === "unstaged" && f.worktree === "?" ? "untracked" : which;
    store.diff = { file: f.path, orig: which === "staged" ? f.origPath : null, mode };
  }

  async function discard(list: FileChange[]) {
    if (!list.length) return;
    const what = list.length === 1 ? list[0].path : `${list.length} files`;
    if (!(await confirm("Discard changes?", `Permanently discard your unstaged changes to ${what}? This can't be undone.`, "Discard", true))) return;
    const untracked = list.filter((f) => f.worktree === "?").map((f) => f.path);
    const tracked = list.filter((f) => f.worktree !== "?").map((f) => f.path);
    if (store.diff && list.some((f) => f.path === store.diff?.file)) store.diff = null;
    await run(() => api.discard(store.path, tracked, untracked));
  }

  function fileMenu(f: FileChange, which: "unstaged" | "staged" | "conflict"): MenuItem[] {
    const items: MenuItem[] = [];
    if (which === "unstaged" || which === "conflict")
      items.push({ label: which === "conflict" ? "Mark as resolved" : "Stage", icon: "plus", action: () => run(() => api.stage(store.path, [f.path])) });
    if (which === "staged") items.push({ label: "Unstage", icon: "minus", action: () => run(() => api.unstage(store.path, [f.path])) });
    if (which === "unstaged") items.push({ label: "Discard changes…", icon: "undo", danger: true, action: () => discard([f]) });
    items.push(
      "sep",
      { label: "Open file", icon: "file", action: () => run(() => openPath(join(store.path, f.path))), disabled: f.worktree === "D" },
      { label: "Show in folder", icon: "folder", action: () => run(() => revealItemInDir(join(store.path, f.path))) },
      { label: "Copy path", icon: "copy", action: () => navigator.clipboard.writeText(f.path).then(() => toast("Path copied")) },
    );
    return items;
  }

  async function toggleAmend() {
    amend = !amend;
    if (amend && !summary.trim()) {
      const msg = await api.headMessage(store.path).catch(() => "");
      const [first, ...rest] = msg.split("\n");
      summary = first ?? "";
      body = rest.join("\n").trim();
    }
  }

  let canCommit = $derived(
    !committing && !!summary.trim() && conflicted.length === 0 && (staged.length > 0 || unstaged.length > 0 || amend),
  );

  async function commit() {
    if (!canCommit) return;
    committing = true;
    try {
      if (!staged.length && unstaged.length && !amend) {
        if (!(await runOk(() => api.stage(store.path, unstaged.map((f) => f.path))))) return;
      }
      const message = body.trim() ? `${summary.trim()}\n\n${body.trim()}` : summary.trim();
      const ok = await runOk(() => api.commit(store.path, message, amend), { done: amend ? "Commit amended" : "Committed" });
      if (ok) {
        summary = "";
        body = "";
        amend = false;
        store.diff = null;
      }
    } finally {
      committing = false;
    }
  }
</script>

<div class="wip">
  <div class="lists">
    {#if conflicted.length}
      <div class="head conflict">
        <span class="section-title">Conflicts ({conflicted.length})</span>
      </div>
      {#each conflicted as f (f.path)}
        <FileRow
          path={f.path}
          status="U"
          selected={isSel(f, "unstaged")}
          onclick={() => showDiff(f, "unstaged")}
          oncontextmenu={(e) => showMenu(e, fileMenu(f, "conflict"))}
        >
          {#snippet actions()}
            <button class="btn small" title="Mark as resolved" onclick={(e) => { e.stopPropagation(); run(() => api.stage(store.path, [f.path])); }}>
              Resolved
            </button>
          {/snippet}
        </FileRow>
      {/each}
    {/if}

    <div class="head">
      <span class="section-title">Unstaged ({unstaged.length})</span>
      <span class="head-actions">
        {#if unstaged.length}
          <button class="btn small ghost" title="Discard all unstaged changes" onclick={() => discard(unstaged)}><Icon name="undo" size={13} /></button>
          <button class="btn small" onclick={() => run(() => api.stage(store.path, unstaged.map((f) => f.path)))}>Stage all</button>
        {/if}
      </span>
    </div>
    {#each unstaged as f (f.path)}
      <FileRow
        path={f.path}
        status={f.worktree}
        selected={isSel(f, f.worktree === "?" ? "untracked" : "unstaged")}
        onclick={() => showDiff(f, "unstaged")}
        oncontextmenu={(e) => showMenu(e, fileMenu(f, "unstaged"))}
      >
        {#snippet actions()}
          <button class="btn ghost icon" title="Discard" onclick={(e) => { e.stopPropagation(); discard([f]); }}><Icon name="undo" size={13} /></button>
          <button class="btn ghost icon" title="Stage" onclick={(e) => { e.stopPropagation(); run(() => api.stage(store.path, [f.path])); }}><Icon name="plus" size={14} /></button>
        {/snippet}
      </FileRow>
    {:else}
      <div class="none">No unstaged changes</div>
    {/each}

    <div class="head">
      <span class="section-title">Staged ({staged.length})</span>
      {#if staged.length}
        <button class="btn small" onclick={() => run(() => api.unstage(store.path, staged.map((f) => f.path)))}>Unstage all</button>
      {/if}
    </div>
    {#each staged as f (f.path)}
      <FileRow
        path={f.path}
        status={f.index}
        selected={isSel(f, "staged")}
        onclick={() => showDiff(f, "staged")}
        oncontextmenu={(e) => showMenu(e, fileMenu(f, "staged"))}
      >
        {#snippet actions()}
          <button class="btn ghost icon" title="Unstage" onclick={(e) => { e.stopPropagation(); run(() => api.unstage(store.path, [f.path])); }}><Icon name="minus" size={14} /></button>
        {/snippet}
      </FileRow>
    {:else}
      <div class="none">Stage files to include them in the next commit</div>
    {/each}
  </div>

  <form
    class="commit"
    onsubmit={(e) => {
      e.preventDefault();
      commit();
    }}
  >
    <div class="summary">
      <input
        placeholder="Commit summary"
        bind:value={summary}
        maxlength="200"
        onkeydown={(e) => (e.ctrlKey || e.metaKey) && e.key === "Enter" && commit()}
      />
      <span class="count" class:over={summary.length > 72}>{summary.length}</span>
    </div>
    <textarea
      placeholder="Description (optional)"
      rows="4"
      bind:value={body}
      onkeydown={(e) => (e.ctrlKey || e.metaKey) && e.key === "Enter" && commit()}
    ></textarea>
    <label class="amend"><input type="checkbox" checked={amend} onchange={toggleAmend} /> Amend previous commit</label>
    <button class="btn primary big" type="submit" disabled={!canCommit}>
      <Icon name="check" size={15} />
      {#if conflicted.length}
        Resolve conflicts first
      {:else if amend}
        Amend commit
      {:else if !staged.length && unstaged.length}
        Stage all & commit
      {:else}
        Commit {staged.length} file{staged.length === 1 ? "" : "s"}
      {/if}
    </button>
    <div class="hint muted">Ctrl+Enter to commit</div>
  </form>
</div>

<style>
  .wip {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .lists {
    flex: 1;
    overflow: auto;
    padding-bottom: 8px;
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 10px 6px 12px;
    position: sticky;
    top: 0;
    background: var(--panel);
    z-index: 1;
  }
  .head.conflict .section-title {
    color: var(--danger);
  }
  .head-actions {
    display: flex;
    gap: 4px;
  }
  .none {
    padding: 4px 12px;
    color: var(--faint);
    font-size: 12px;
  }
  .commit {
    display: flex;
    flex-direction: column;
    gap: 8px;
    padding: 12px;
    border-top: 1px solid var(--border);
  }
  .summary {
    position: relative;
  }
  .summary input {
    padding-right: 38px;
    font-weight: 500;
  }
  .count {
    position: absolute;
    right: 9px;
    top: 50%;
    transform: translateY(-50%);
    font-size: 11px;
    color: var(--faint);
  }
  .count.over {
    color: var(--warn);
  }
  textarea {
    resize: vertical;
    min-height: 60px;
  }
  .amend {
    display: flex;
    align-items: center;
    gap: 7px;
    color: var(--muted);
  }
  .big {
    padding: 9px;
    font-weight: 600;
  }
  .hint {
    font-size: 11px;
    text-align: center;
  }
</style>
