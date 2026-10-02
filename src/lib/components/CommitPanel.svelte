<script lang="ts">
  import { api, type CommitInfo } from "../api";
  import type { RepoStore } from "../repo.svelte";
  import { showMenu } from "../menu.svelte";
  import { errorText, toast } from "../state.svelte";
  import { fullDate, hue, initials, relativeTime, shortHash } from "../util";
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
    <div class="empty">{error}</div>
  {:else if info}
    <div class="top">
      {#if stash}<div class="kind"><Icon name="stash" size={13} /> Stash</div>{/if}
      <h2 class="selectable">{subject}</h2>
      {#if rest}<pre class="body selectable">{rest}</pre>{/if}

      <div class="who">
        <span class="avatar" style="background:hsl({hue(info.email)} 55% 45%)">{initials(info.author)}</span>
        <div class="meta">
          <div><strong>{info.author}</strong> <span class="muted">{info.email}</span></div>
          <div class="muted" title={fullDate(info.time)}>
            authored {relativeTime(info.time)}{info.committer !== info.author ? ` · committed by ${info.committer}` : ""}
          </div>
        </div>
      </div>

      <div class="ids">
        <button class="btn small mono" title="Copy full hash" onclick={copyHash}>
          <Icon name="copy" size={12} />
          {shortHash(info.hash)}
        </button>
        {#each info.parents as p (p)}
          <button class="btn small ghost mono" title="Go to parent" onclick={() => store.selectCommit(p)}>
            ↑ {shortHash(p)}
          </button>
        {/each}
        {#if commit}
          <button class="btn small ghost icon more" title="More actions" onclick={(e) => showMenu(e, store.commitMenu(commit))}>
            <Icon name="more" size={16} stroke={3} />
          </button>
        {/if}
      </div>
    </div>

    <div class="files-head">
      <span class="section-title">{info.files.length} file{info.files.length === 1 ? "" : "s"} changed</span>
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
    <div class="empty">Loading…</div>
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
    padding: 14px 14px 10px;
    display: flex;
    flex-direction: column;
    gap: 10px;
    border-bottom: 1px solid var(--border);
    max-height: 55%;
    overflow: auto;
  }
  .kind {
    display: flex;
    align-items: center;
    gap: 5px;
    font-size: 11px;
    font-weight: 600;
    color: var(--accent);
    text-transform: uppercase;
    letter-spacing: 0.05em;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    line-height: 1.4;
    word-break: break-word;
  }
  .body {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font: inherit;
    color: var(--muted);
  }
  .who {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .avatar {
    width: 32px;
    height: 32px;
    border-radius: 50%;
    display: grid;
    place-items: center;
    color: #fff;
    font-weight: 600;
    font-size: 12px;
    flex-shrink: 0;
  }
  .meta {
    min-width: 0;
    font-size: 12px;
    overflow: hidden;
  }
  .meta > div {
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .ids {
    display: flex;
    gap: 4px;
    flex-wrap: wrap;
    align-items: center;
  }
  .more {
    margin-left: auto;
  }
  .files-head {
    padding: 10px 12px 6px;
  }
  .files {
    flex: 1;
    overflow: auto;
  }
</style>
