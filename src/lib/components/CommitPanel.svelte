<script lang="ts">
  import { api, type CommitInfo } from "../api";
  import type { RepoStore } from "../repo.svelte";
  import { showMenu } from "../menu.svelte";
  import { errorText, toast } from "../state.svelte";
  import { fullDate, initials, relativeTime, shortHash } from "../util";
  import FileRow from "./FileRow.svelte";
  import Icon from "./Icon.svelte";

  let { store, hash }: { store: RepoStore; hash: string } = $props();

  let info = $state<CommitInfo | null>(null);
  let error = $state("");

  $effect(() => {
    let cancelled = false;
    api
      .commitInfo(store.path, hash)
      .then((i) => !cancelled && (info = i))
      .catch((e) => !cancelled && (error = errorText(e)));
    return () => (cancelled = true);
  });

  let subject = $derived(info?.message.split("\n")[0] ?? "");
  let rest = $derived(info?.message.split("\n").slice(1).join("\n").trim() ?? "");
  let commit = $derived(store.commits.find((c) => c.hash === hash));
  let stash = $derived(store.stashes.find((s) => s.hash === hash));

  function copyHash() {
    void navigator.clipboard.writeText(hash).then(() => toast("Hash copied"));
  }
</script>

<div class="panel">
  {#if error}
    <div class="empty-note selectable">{error}</div>
  {:else if info}
    <div class="top">
      {#if stash}<span class="k-badge accent kind"><Icon name="stash" size={14} /> Stash</span>{/if}
      <h2 class="selectable">{subject}</h2>
      {#if rest}<pre class="body selectable">{rest}</pre>{/if}

      <div class="who">
        <span class="avatar" aria-hidden="true">{initials(info.author)}</span>
        <div class="meta">
          <div><strong>{info.author}</strong> <span class="email selectable">{info.email}</span></div>
          <div class="when" title={fullDate(info.time)}>
            Authored {relativeTime(info.time)}{info.committer !== info.author ? ` · committed by ${info.committer}` : ""}
          </div>
        </div>
      </div>

      <div class="ids">
        <button class="k-btn hash" title="Copy full hash" onclick={copyHash}>
          <Icon name="copy" />
          {shortHash(info.hash)}
        </button>
        {#each info.parents as p (p)}
          <button class="k-btn ghost hash" title="Go to parent {shortHash(p)}" onclick={() => store.selectCommit(p)}>
            <Icon name="commit" />
            {shortHash(p)}
          </button>
        {/each}
        {#if commit}
          <button class="k-icon-btn more" aria-label="More actions" title="More actions" onclick={(e) => showMenu(e, store.commitMenu(commit))}>
            <Icon name="more" />
          </button>
        {/if}
      </div>
    </div>

    <div class="files-head">
      <span class="k-label">{info.files.length} file{info.files.length === 1 ? "" : "s"} changed</span>
    </div>
    <div class="files">
      {#each info.files as f (f.path)}
        <FileRow
          path={f.path}
          status={f.status}
          selected={store.diff?.file === f.path && store.diff?.hash === hash}
          onclick={() => (store.diff = { file: f.path, orig: f.origPath, mode: "commit", hash })}
        />
      {/each}
    </div>
  {:else}
    <div class="empty-note"><span class="k-spinner" role="status" aria-label="Loading commit"></span></div>
  {/if}
</div>

<style>
  .panel {
    display: flex;
    flex-direction: column;
    height: 100%;
    min-height: 0;
  }
  .top {
    padding: var(--space-16) var(--space-16) var(--space-12);
    display: flex;
    flex-direction: column;
    gap: var(--space-12);
    border-bottom: 1px solid var(--border);
    max-height: 55%;
    overflow: auto;
  }
  .kind {
    align-self: flex-start;
  }
  h2 {
    font-size: 15px;
    line-height: 20px;
    font-weight: 600;
    word-break: break-word;
  }
  .body {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font: inherit;
    color: var(--text-2);
  }
  .who {
    display: flex;
    gap: var(--space-12);
    align-items: center;
  }
  .avatar {
    width: 32px;
    height: 32px;
    border-radius: var(--radius-full);
    display: grid;
    place-items: center;
    background: var(--surface-2);
    color: var(--text-2);
    font-weight: 600;
    font-size: 12px;
    flex-shrink: 0;
  }
  .meta {
    min-width: 0;
    font-size: 12px;
    line-height: 16px;
    overflow: hidden;
  }
  .meta > div {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .meta strong {
    font-size: 13px;
    font-weight: 600;
  }
  .email,
  .when {
    color: var(--text-3);
  }
  .ids {
    display: flex;
    gap: var(--space-4);
    flex-wrap: wrap;
    align-items: center;
  }
  .hash {
    font-family: var(--font-mono);
    font-size: 12px;
  }
  .more {
    margin-left: auto;
  }
  .files-head {
    padding: var(--space-12) var(--space-16) var(--space-6);
  }
  .files {
    flex: 1;
    overflow: auto;
    padding-bottom: var(--space-8);
  }
  .empty-note .k-spinner {
    display: inline-block;
  }
</style>
