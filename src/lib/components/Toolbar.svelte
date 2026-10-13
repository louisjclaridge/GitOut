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
    <Icon name="branch" />
    {#if st?.branch}
      <strong class="ellipsis" title={st.branch}>{st.branch}</strong>
      {#if st.upstream}
        <span class="upstream ellipsis" title="Tracking {st.upstream}">{st.upstream}</span>
      {:else if store.loaded}
        <span class="upstream">Not published</span>
      {/if}
    {:else if st}
      <strong>Detached HEAD</strong>
    {/if}
  </div>

  <div class="group">
    <button class="k-btn ghost" disabled={busy} title="Fetch from all remotes" onclick={() => store.fetch()}>
      <Icon name="fetch" />Fetch
    </button>
    <button class="k-btn ghost" disabled={busy || !st?.upstream} title="Pull from {st?.upstream ?? 'upstream'}" onclick={() => store.pull()}>
      <Icon name="pull" />Pull
      {#if st?.behind}<span class="k-badge accent" title="{st.behind} to pull">{st.behind}</span>{/if}
    </button>
    <button
      class="k-btn ghost"
      disabled={busy || !st?.branch}
      title={st?.upstream ? `Push to ${st.upstream}` : "Publish this branch"}
      onclick={() => store.push()}
      oncontextmenu={(e) =>
        showMenu(e, [{ label: "Force push (with lease)…", icon: "warning", danger: true, action: () => store.push(true) }])}
    >
      <Icon name="push" />{st?.upstream || !store.loaded ? "Push" : "Publish"}
      {#if st?.ahead}<span class="k-badge accent" title="{st.ahead} to push">{st.ahead}</span>{/if}
    </button>
  </div>

  <span class="divider"></span>

  <div class="group">
    <button class="k-btn ghost" title="Create a branch" onclick={() => store.createBranch()}>
      <Icon name="branch" />Branch
    </button>
    <button class="k-btn ghost" disabled={!store.dirty} title="Stash your uncommitted changes" onclick={() => store.stash()}>
      <Icon name="stash" />Stash
    </button>
    <button
      class="k-btn ghost"
      disabled={!stashCount}
      title="Apply and remove the latest stash"
      onclick={() => store.popLatest()}
      oncontextmenu={(e) => store.stashes[0] && showMenu(e, store.stashMenu(store.stashes[0]))}
    >
      <Icon name="pop" />Pop
      {#if stashCount}<span class="k-badge" title="{stashCount} stashed">{stashCount}</span>{/if}
    </button>
  </div>

  <div class="spacer"></div>

  <div class="group">
    <button class="k-icon-btn" aria-label="Open folder" title="Open folder" onclick={() => store.openFolder()}>
      <Icon name="folder-open" />
    </button>
    <button class="k-icon-btn" aria-label="Refresh" title="Refresh (Ctrl+R)" onclick={() => store.load()}>
      <Icon name="refresh" />
    </button>
  </div>
</div>

<style>
  .toolbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--space-8);
    height: 48px;
    padding: 0 var(--space-8) 0 var(--space-16);
    background: var(--bg);
    border-bottom: 1px solid var(--border);
    overflow: hidden;
  }
  .where {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    width: 200px;
    min-width: 0;
    flex-shrink: 0;
    color: var(--accent-text);
  }
  .where strong {
    color: var(--text);
    font-weight: 600;
    flex-shrink: 1;
  }
  .upstream {
    font-size: 12px;
    color: var(--text-3);
    flex-shrink: 2;
  }
  .group {
    display: flex;
    gap: var(--space-4);
  }
  .divider {
    width: 1px;
    height: 20px;
    background: var(--border);
    flex: none;
  }
  .spacer {
    flex: 1;
  }
  .k-btn {
    padding: 0 var(--space-8);
  }
  .k-btn .k-badge {
    height: 18px;
    min-width: 18px;
    justify-content: center;
    padding: 0 5px;
    font-size: 11px;
    font-variant-numeric: tabular-nums;
  }
  @media (max-width: 1100px) {
    .where {
      width: 168px;
    }
  }
</style>
