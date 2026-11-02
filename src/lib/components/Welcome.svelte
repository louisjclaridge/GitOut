<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "../api";
  import { app, addTab, openRepo, run, selectTab, updateSettings } from "../state.svelte";
  import { sync, syncAll } from "../sync.svelte";
  import { basename } from "../util";
  import Icon from "./Icon.svelte";
  import CloneDialog from "./CloneDialog.svelte";
  import ProviderLogo from "./ProviderLogo.svelte";

  let cloning = $state(false);

  async function pickAndOpen() {
    const dir = await open({ directory: true, title: "Open a git repository" });
    if (typeof dir === "string") await openRepo(dir);
  }

  async function pickAndInit() {
    const dir = await open({ directory: true, title: "Choose a folder for the new repository" });
    if (typeof dir !== "string") return;
    const info = await run(() => api.repoInit(dir), { done: "Repository created" });
    if (info) addTab(info);
  }

  function forget(path: string) {
    void updateSettings({ recentRepos: (app.settings?.recentRepos ?? []).filter((r) => r !== path) });
  }

  const STEP_ICON: Record<string, string> = { done: "success", error: "error", skipped: "skipped", waiting: "queued" };
  const STEP_TEXT: Record<string, string> = { fetching: "Fetching…", pulling: "Pulling…", waiting: "Waiting…" };
</script>

<div class="welcome">
  <div class="inner">
    <div class="brand">
      <img src="/logo.svg" alt="" width="44" height="44" />
      <div>
        <h1 class="k-h1">GitOut</h1>
        <p>A small, fast git client.</p>
      </div>
    </div>

    <div class="actions">
      <button class="k-card row" onclick={pickAndOpen}>
        <span class="k-tile lg"><Icon name="folder-open" size={20} /></span>
        <span class="text">
          <span class="title">Open…</span>
          <span class="sub">A repository on this computer</span>
        </span>
      </button>
      <button class="k-card row" onclick={() => (cloning = true)}>
        <span class="k-tile lg"><Icon name="download" size={20} /></span>
        <span class="text">
          <span class="title">Clone…</span>
          <span class="sub">From GitHub, GitLab, Bitbucket or a URL</span>
        </span>
      </button>
      <button class="k-card row" onclick={pickAndInit}>
        <span class="k-tile lg"><Icon name="folder-plus" size={20} /></span>
        <span class="text">
          <span class="title">Create…</span>
          <span class="sub">A new, empty repository</span>
        </span>
      </button>
    </div>

    {#if app.tabs.length}
      <section class="k-group-section">
        <div class="open-head">
          <div>
            <h2 class="k-group-header"><Icon name="folder-open" /> Open repositories</h2>
            <p class="k-group-desc">Fetch every one, and pull those that are behind.</p>
          </div>
          <button class="k-btn primary" disabled={sync.running || app.busy !== null} onclick={syncAll}>
            {#if sync.running}<span class="k-spinner"></span> Syncing…{:else}<Icon name="sync" /> Fetch &amp; pull all{/if}
          </button>
        </div>
        <div class="k-group">
          {#each app.tabs as t (t.path)}
            {@const item = sync.items[t.path]}
            {@const busy = item?.step === "fetching" || item?.step === "pulling"}
            <button class="k-group-row repo-row" title={t.path} onclick={() => selectTab(t.path)}>
              <span class="state {item?.step ?? 'idle'}">
                {#if busy}
                  <span class="k-spinner" role="status" aria-label={STEP_TEXT[item.step]}></span>
                {:else}
                  <Icon name={STEP_ICON[item?.step ?? ""] ?? "folder"} />
                {/if}
              </span>
              <span class="label">
                <span class="ellipsis">{t.name}</span>
                {#if item && (STEP_TEXT[item.step] || item.detail)}
                  <span class="desc" class:err={item.step === "error"}>{STEP_TEXT[item.step] ?? item.detail}</span>
                {:else}
                  <span class="desc mono ellipsis">{t.path}</span>
                {/if}
              </span>
              <Icon name="chevron" size={14} />
            </button>
          {/each}
        </div>
      </section>
    {/if}

    <div class="columns">
      <section class="k-group-section">
        <h2 class="k-group-header"><Icon name="folder" /> Recent</h2>
        <div class="k-group">
          {#each app.settings?.recentRepos ?? [] as r (r)}
            <div class="recent">
              <button class="recent-row" onclick={() => openRepo(r)} title={r}>
                <span class="name">{basename(r)}</span>
                <span class="path mono ellipsis">{r}</span>
              </button>
              <button class="k-icon-btn forget" aria-label="Remove {basename(r)} from list" title="Remove from list" onclick={() => forget(r)}>
                <Icon name="x" />
              </button>
            </div>
          {:else}
            <div class="k-group-row note">Repositories you open will show up here.</div>
          {/each}
        </div>
      </section>

      <section class="k-group-section">
        <h2 class="k-group-header"><Icon name="user" /> Accounts</h2>
        <div class="k-group">
          {#each app.settings?.accounts ?? [] as a (a.id)}
            <div class="k-group-row acct">
              <ProviderLogo provider={a.provider} size={20} />
              <span class="label">
                <span class="ellipsis">{a.displayName || a.username}</span>
                <span class="desc ellipsis">{a.host}</span>
              </span>
            </div>
          {:else}
            <div class="k-group-row note">Sign in to clone your repositories and push over HTTPS without passwords.</div>
          {/each}
        </div>
        <p class="k-group-footer">
          <button class="k-link" onclick={() => (app.view = "settings")}>Manage accounts…</button>
        </p>
      </section>
    </div>
  </div>
</div>

{#if cloning}
  <CloneDialog onclose={() => (cloning = false)} />
{/if}

<style>
  .welcome {
    height: 100%;
    overflow: auto;
    display: flex;
    justify-content: center;
    padding: var(--space-48) var(--space-24);
    background: var(--bg);
  }
  .inner {
    width: min(860px, 100%);
    display: flex;
    flex-direction: column;
    gap: var(--space-24);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-16);
  }
  .brand p {
    color: var(--text-3);
  }
  .actions {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
    gap: var(--space-12);
  }
  .open-head {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    margin-bottom: var(--space-8);
  }
  .open-head > div {
    flex: 1;
    min-width: 0;
  }
  .repo-row {
    width: 100%;
    border: 0;
    background: none;
    color: var(--text);
    font: inherit;
    text-align: left;
    cursor: pointer;
  }
  .repo-row + .repo-row {
    border-top: 1px solid var(--border);
  }
  .repo-row:hover .label > .ellipsis {
    color: var(--accent-text);
  }
  .repo-row > :global(.k-icon) {
    color: var(--text-3);
  }
  .state {
    display: grid;
    place-items: center;
    width: 32px;
    height: 32px;
    flex: none;
    border-radius: var(--radius-sm);
    background: var(--surface-2);
    color: var(--text-3);
  }
  .state.done {
    background: var(--success-soft);
    color: var(--success-text);
  }
  .state.error {
    background: var(--danger-soft);
    color: var(--danger-text);
  }
  .state.fetching,
  .state.pulling {
    background: var(--accent-soft);
  }
  .desc.err {
    color: var(--danger-text);
    white-space: pre-wrap;
    word-break: break-word;
    user-select: text;
  }
  .columns {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: var(--space-16);
    align-items: start;
  }
  @media (max-width: 760px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }
  .k-group-section + .k-group-section {
    margin-top: 0;
  }
  .recent {
    display: flex;
    align-items: center;
    margin: 0 calc(-1 * var(--space-8));
    border-radius: var(--radius-sm);
  }
  .recent:hover {
    background: var(--surface-hover);
  }
  .recent-row {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: var(--space-12);
    min-width: 0;
    min-height: 40px;
    padding: var(--space-8);
    border: 0;
    background: none;
    color: var(--text);
    text-align: left;
  }
  .name {
    font-weight: 600;
    flex-shrink: 0;
  }
  .path {
    color: var(--text-3);
  }
  .forget {
    visibility: hidden;
  }
  .recent:hover .forget,
  .forget:focus-visible {
    visibility: visible;
  }
  .note {
    color: var(--text-3);
  }
  .acct .label {
    min-width: 0;
  }
</style>
