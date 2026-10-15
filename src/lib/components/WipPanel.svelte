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
    if (which === "conflict") {
      // During a rebase, git's "ours" is the upstream and "theirs" is your commit.
      const [oursName, theirsName] = store.status?.state === "rebasing" ? ["upstream", "your commit's"] : ["current", "incoming"];
      items.push(
        { label: "Resolve in merge editor", icon: "merge", action: () => store.openMerge(f.path) },
        { label: `Keep ${oursName} version`, icon: "back", action: () => run(() => api.conflictTake(store.path, f.path, "ours"), { done: `Resolved ${f.path}` }) },
        { label: `Keep ${theirsName} version`, icon: "forward", action: () => run(() => api.conflictTake(store.path, f.path, "theirs"), { done: `Resolved ${f.path}` }) },
        { label: "Show diff", icon: "file", action: () => showDiff(f, "unstaged") },
        "sep",
      );
    }
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
        <span class="k-label"><Icon name="warning" size={14} /> Conflicts <span class="n">{conflicted.length}</span></span>
      </div>
      {#each conflicted as f (f.path)}
        <FileRow
          path={f.path}
          status="U"
          selected={(store.center.kind === "merge" && store.center.file === f.path) || isSel(f, "unstaged")}
          onclick={() => store.openMerge(f.path)}
          oncontextmenu={(e) => showMenu(e, fileMenu(f, "conflict"))}
        >
          {#snippet actions()}
            <button class="k-btn" title="Resolve in the merge editor" onclick={(e) => { e.stopPropagation(); store.openMerge(f.path); }}>
              Resolve
            </button>
          {/snippet}
        </FileRow>
      {/each}
    {/if}

    <div class="head">
      <span class="k-label">Unstaged <span class="n">{unstaged.length}</span></span>
      <span class="head-actions">
        {#if unstaged.length}
          <button class="k-icon-btn" aria-label="Discard all unstaged changes" title="Discard all unstaged changes" onclick={() => discard(unstaged)}><Icon name="undo" /></button>
          <button class="k-btn" onclick={() => run(() => api.stage(store.path, unstaged.map((f) => f.path)))}>Stage all</button>
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
          <button class="k-icon-btn" aria-label="Discard changes" title="Discard changes" onclick={(e) => { e.stopPropagation(); discard([f]); }}><Icon name="undo" /></button>
          <button class="k-icon-btn" aria-label="Stage" title="Stage" onclick={(e) => { e.stopPropagation(); run(() => api.stage(store.path, [f.path])); }}><Icon name="plus" /></button>
        {/snippet}
      </FileRow>
    {:else}
      <div class="none">No unstaged changes</div>
    {/each}

    <div class="head">
      <span class="k-label">Staged <span class="n">{staged.length}</span></span>
      {#if staged.length}
        <button class="k-btn" onclick={() => run(() => api.unstage(store.path, staged.map((f) => f.path)))}>Unstage all</button>
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
          <button class="k-icon-btn" aria-label="Unstage" title="Unstage" onclick={(e) => { e.stopPropagation(); run(() => api.unstage(store.path, [f.path])); }}><Icon name="minus" /></button>
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
        class="k-input"
        placeholder="Commit summary"
        aria-label="Commit summary"
        bind:value={summary}
        maxlength="200"
        onkeydown={(e) => (e.ctrlKey || e.metaKey) && e.key === "Enter" && commit()}
      />
      <span class="count" class:over={summary.length > 72} title="72 characters or fewer reads best">{summary.length}</span>
    </div>
    <textarea
      class="k-input"
      placeholder="Description (optional)"
      aria-label="Description"
      rows="4"
      bind:value={body}
      onkeydown={(e) => (e.ctrlKey || e.metaKey) && e.key === "Enter" && commit()}
    ></textarea>
    <label class="k-check"><input type="checkbox" checked={amend} onchange={toggleAmend} /><span>Amend previous commit</span></label>
    <button class="k-btn primary block" type="submit" disabled={!canCommit}>
      {#if committing}<span class="k-spinner"></span>{:else}<Icon name="check" />{/if}
      {#if conflicted.length}
        Resolve conflicts first
      {:else if amend}
        Amend commit
      {:else if !staged.length && unstaged.length}
        Stage all and commit
      {:else}
        Commit {staged.length} file{staged.length === 1 ? "" : "s"}
      {/if}
    </button>
    <div class="hint"><kbd class="k-kbd">Ctrl</kbd> <kbd class="k-kbd">Enter</kbd> to commit</div>
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
    padding-bottom: var(--space-8);
  }
  .head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-8);
    min-height: 44px;
    padding: var(--space-8) var(--space-8) var(--space-4) var(--space-16);
    position: sticky;
    top: 0;
    background: var(--surface);
    z-index: 1;
  }
  .head .k-label {
    display: inline-flex;
    align-items: center;
    gap: var(--space-6);
  }
  .head .n {
    font-weight: 400;
    font-variant-numeric: tabular-nums;
  }
  .head.conflict .k-label {
    color: var(--danger);
  }
  .head-actions {
    display: flex;
    gap: var(--space-4);
  }
  .none {
    padding: var(--space-4) var(--space-16);
    color: var(--text-3);
    font-size: 12px;
  }
  .commit {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    padding: var(--space-12);
    border-top: 1px solid var(--border);
  }
  .summary {
    position: relative;
  }
  .summary .k-input {
    padding-right: 40px;
    font-weight: 500;
  }
  .count {
    position: absolute;
    right: var(--space-12);
    top: 50%;
    transform: translateY(-50%);
    font-size: 12px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .count.over {
    color: var(--warning);
  }
  textarea {
    min-height: 64px;
  }
  .k-check {
    color: var(--text-2);
  }
  .k-btn.primary {
    font-weight: 600;
  }
  .hint {
    font-size: 12px;
    line-height: 16px;
    text-align: center;
    color: var(--text-3);
  }
</style>
