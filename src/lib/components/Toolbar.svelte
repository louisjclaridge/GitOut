<script lang="ts">
  import type { RepoStore } from "../repo.svelte";
  import { app } from "../state.svelte";
  import { showMenu } from "../menu.svelte";
  import Icon from "./Icon.svelte";

  let { store }: { store: RepoStore } = $props();

  let busy = $derived(app.busy !== null);
  let st = $derived(store.status);
  let stashCount = $derived(store.stashes.length);
</script>

<div class="toolbar">
  <div class="where">
    <Icon name="branch" size={15} />
    {#if st?.branch}
      <strong class="ellipsis">{st.branch}</strong>
      {#if st.upstream}
        <span class="muted ellipsis upstream">→ {st.upstream}</span>
      {:else if store.loaded}
        <span class="muted">(not published)</span>
      {/if}
    {:else if st}
      <strong>detached HEAD</strong>
    {/if}
  </div>

  <div class="group">
    <button class="tb" disabled={busy} title="Fetch from all remotes" onclick={() => store.fetch()}>
      <Icon name="fetch" size={17} /><span>Fetch</span>
    </button>
    <button class="tb" disabled={busy || !st?.upstream} title="Pull from {st?.upstream ?? 'upstream'}" onclick={() => store.pull()}>
      <Icon name="pull" size={17} /><span>Pull</span>
      {#if st?.behind}<em class="count">{st.behind}</em>{/if}
    </button>
    <button
      class="tb"
      disabled={busy || !st?.branch}
      title={st?.upstream ? `Push to ${st.upstream}` : "Publish this branch"}
      onclick={() => store.push()}
      oncontextmenu={(e) =>
        showMenu(e, [{ label: "Force push (with lease)…", icon: "warning", danger: true, action: () => store.push(true) }])}
    >
      <Icon name="push" size={17} /><span>{st?.upstream || !store.loaded ? "Push" : "Publish"}</span>
      {#if st?.ahead}<em class="count">{st.ahead}</em>{/if}
    </button>
  </div>

  <div class="group">
    <button class="tb" title="Create a branch" onclick={() => store.createBranch()}>
      <Icon name="branch" size={17} /><span>Branch</span>
    </button>
    <button class="tb" disabled={!store.dirty} title="Stash your uncommitted changes" onclick={() => store.stash()}>
      <Icon name="stash" size={17} /><span>Stash</span>
    </button>
    <button
      class="tb"
      disabled={!stashCount}
      title="Apply and remove the latest stash"
      onclick={() => store.popLatest()}
      oncontextmenu={(e) => store.stashes[0] && showMenu(e, store.stashMenu(store.stashes[0]))}
    >
      <Icon name="pop" size={17} /><span>Pop</span>
      {#if stashCount}<em class="count">{stashCount}</em>{/if}
    </button>
  </div>

  <div class="spacer"></div>

  <button class="tb" title="Open folder" onclick={() => store.openFolder()}>
    <Icon name="folder" size={17} />
  </button>
  <button class="tb" title="Refresh (Ctrl+R)" onclick={() => store.load()}>
    <Icon name="fetch" size={17} />
  </button>
</div>

<style>
  .toolbar {
    display: flex;
    align-items: center;
    gap: 10px;
    height: 54px;
    padding: 0 12px;
    background: var(--panel);
    border-bottom: 1px solid var(--border);
  }
  .where {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 206px;
    min-width: 0;
    flex-shrink: 0;
  }
  .upstream {
    font-size: 12px;
  }
  .group {
    display: flex;
    gap: 2px;
    padding-left: 10px;
    border-left: 1px solid var(--border);
  }
  .spacer {
    flex: 1;
  }
  .tb {
    position: relative;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    gap: 2px;
    min-width: 48px;
    height: 44px;
    padding: 0 8px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--text);
  }
  .tb span {
    font-size: 11px;
    color: var(--muted);
  }
  .tb:hover:not(:disabled) {
    background: var(--hover);
  }
  .tb:disabled {
    opacity: 0.4;
    cursor: default;
  }
  .count {
    position: absolute;
    top: 3px;
    right: 6px;
    min-width: 16px;
    height: 16px;
    padding: 0 4px;
    border-radius: 8px;
    font-size: 10px;
    font-style: normal;
    font-weight: 600;
    line-height: 16px;
    text-align: center;
    background: var(--accent);
    color: var(--accent-text);
  }
</style>
