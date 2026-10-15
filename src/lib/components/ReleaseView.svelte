<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { RepoStore } from "../repo.svelte";
  import { showMenu } from "../menu.svelte";
  import { run } from "../state.svelte";
  import { formatBytes, fullDate, isoToUnix, relativeTime, shortHash } from "../util";
  import Icon from "./Icon.svelte";

  let { store, id }: { store: RepoStore; id: string } = $props();

  let release = $derived(store.releases.find((r) => r.id === id));
  let tag = $derived(release ? store.tagRef(release.tag) : undefined);
  let published = $derived(isoToUnix(release?.publishedAt));
  let created = $derived(isoToUnix(release?.createdAt));
  let host = $derived(store.releaseHost);
</script>

<div class="release">
  <header>
    <button class="k-icon-btn" aria-label="Back to history" title="Back to history (Esc)" onclick={() => (store.center = { kind: "graph" })}>
      <Icon name="back" />
    </button>
    {#if release}
      <div class="title">
        <strong class="ellipsis selectable">{release.name || release.tag}</strong>
        {#if host}<span class="ellipsis">{host.project} on {host.host}</span>{/if}
      </div>
      {#if release.draft}<span class="k-badge warning">Draft</span>{/if}
      {#if release.prerelease}<span class="k-badge accent">Pre-release</span>{/if}
      <button class="k-btn" onclick={() => store.editRelease(release)}><Icon name="pencil" /> Edit</button>
      <button class="k-icon-btn" aria-label="More actions" title="More actions" onclick={(e) => showMenu(e, store.releaseMenu(release))}>
        <Icon name="more" />
      </button>
    {:else}
      <div class="title"><strong>Release</strong></div>
    {/if}
  </header>

  <div class="content">
    {#if !release}
      <div class="empty-note">
        {#if store.releasesStatus === "loading"}
          <span class="k-spinner" role="status" aria-label="Loading release"></span>
        {:else}
          This release no longer exists.
        {/if}
      </div>
    {:else}
      <div class="meta">
        <button
          class="k-btn ghost mono"
          title={tag ? `Show the commit ${release.tag} points to` : `${release.tag} isn't on this computer yet. Fetch to get it.`}
          disabled={!tag}
          onclick={() => tag && store.selectCommit(tag.target)}
        >
          <Icon name="tag" />
          {release.tag}
          {#if tag}<span class="k-muted">{shortHash(tag.target)}</span>{/if}
        </button>
        <span class="when" title={published ? fullDate(published) : created ? fullDate(created) : undefined}>
          {#if published && !release.draft}
            Published {relativeTime(published)}
          {:else if created}
            Not published · created {relativeTime(created)}
          {:else}
            Not published
          {/if}
          {#if release.author}by {release.author}{/if}
        </span>
      </div>

      {#if release.body.trim()}
        <pre class="notes selectable">{release.body.trim()}</pre>
      {:else}
        <p class="k-muted">No release notes.</p>
      {/if}

      {#if release.assets.length}
        <div class="files">
          <span class="k-label">Files {release.assets.length}</span>
          <div class="k-list flush">
            {#each release.assets as a (a.url)}
              <button class="k-item" title="Download {a.name} in your browser" onclick={() => run(() => openUrl(a.url))}>
                <Icon name="download" size={14} />
                <span class="name">{a.name}</span>
                <span class="meta">
                  {[a.size !== null ? formatBytes(a.size) : "", a.downloads !== null ? `${a.downloads.toLocaleString()} downloads` : ""]
                    .filter(Boolean)
                    .join(" · ")}
                </span>
              </button>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .release {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    min-height: 48px;
    padding: var(--space-6) var(--space-8);
    border-bottom: 1px solid var(--border);
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title strong {
    font-weight: 600;
  }
  .title span {
    font-size: 11px;
    line-height: 16px;
    color: var(--text-3);
  }
  .content {
    flex: 1;
    overflow: auto;
    padding: var(--space-16) var(--space-24) var(--space-32);
    display: flex;
    flex-direction: column;
    gap: var(--space-16);
    max-width: 820px;
  }
  .meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-8);
    margin-left: calc(-1 * var(--space-12));
  }
  .when {
    color: var(--text-3);
    font-size: 12px;
  }
  .notes {
    margin: 0;
    white-space: pre-wrap;
    word-break: break-word;
    font: inherit;
    line-height: 22px;
  }
  .files {
    display: flex;
    flex-direction: column;
    gap: var(--space-6);
  }
  .k-list.flush {
    padding: 0;
    margin-left: calc(-1 * var(--space-8));
  }
  .empty-note .k-spinner {
    display: inline-block;
  }
</style>
