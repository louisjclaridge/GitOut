<script lang="ts">
  import { open } from "@tauri-apps/plugin-dialog";
  import { api } from "../api";
  import { app, addTab, openRepo, run, updateSettings } from "../state.svelte";
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
</script>

<div class="welcome">
  <div class="inner">
    <div class="brand">
      <img src="/logo.svg" alt="" width="48" height="48" />
      <div>
        <h1>GitOut</h1>
        <p>A small, fast git client.</p>
      </div>
    </div>

    <div class="actions">
      <button class="card" onclick={pickAndOpen}>
        <span class="art"><Icon name="folder-open" size={20} /></span>
        <span class="what">
          <strong>Open…</strong>
          <span class="desc">A repository on this computer</span>
        </span>
      </button>
      <button class="card" onclick={() => (cloning = true)}>
        <span class="art"><Icon name="download" size={20} /></span>
        <span class="what">
          <strong>Clone…</strong>
          <span class="desc">From GitHub, GitLab, Bitbucket or a URL</span>
        </span>
      </button>
      <button class="card" onclick={pickAndInit}>
        <span class="art"><Icon name="folder-plus" size={20} /></span>
        <span class="what">
          <strong>Create…</strong>
          <span class="desc">A new, empty repository</span>
        </span>
      </button>
    </div>

    <div class="columns">
      <section class="k-group-section">
        <h2 class="k-group-header"><Icon name="folder" /> Recent</h2>
        <div class="k-group">
          {#each app.settings?.recentRepos ?? [] as r (r)}
            <div class="recent">
              <button class="row" onclick={() => openRepo(r)} title={r}>
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
    gap: var(--space-32);
  }
  .brand {
    display: flex;
    align-items: center;
    gap: var(--space-16);
  }
  h1 {
    font-size: 28px;
    line-height: 34px;
    font-weight: 700;
    letter-spacing: -0.02em;
  }
  .brand p {
    color: var(--text-2);
  }
  .actions {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--space-12);
  }
  .card {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    padding: var(--space-16);
    text-align: left;
    color: var(--text);
    background: var(--surface);
    border: 0;
    border-radius: var(--radius-md);
    transition: background-color 120ms ease;
  }
  :global([data-theme="dark"]) .card {
    box-shadow: 0 0 0 1px var(--border);
  }
  .card:hover {
    background: var(--surface-hover);
  }
  .card:active {
    opacity: 0.8;
  }
  .art {
    display: grid;
    place-items: center;
    width: 40px;
    height: 40px;
    flex: none;
    border-radius: var(--radius-md);
    background: var(--surface-2);
    color: var(--accent-text);
  }
  .card:hover .art {
    background: var(--surface);
  }
  .what {
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .what strong {
    font-weight: 600;
  }
  .desc {
    font-size: 12px;
    line-height: 16px;
    color: var(--text-3);
  }
  .columns {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: var(--space-24);
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
  .k-group {
    overflow: hidden;
  }
  .recent {
    display: flex;
    align-items: center;
    padding-right: var(--space-8);
  }
  .recent + .recent {
    border-top: 1px solid var(--border);
  }
  .recent:hover {
    background: var(--surface-hover);
  }
  .row {
    flex: 1;
    display: flex;
    align-items: baseline;
    gap: var(--space-12);
    min-width: 0;
    min-height: 44px;
    padding: var(--space-12) var(--space-16);
    border: 0;
    background: none;
    color: var(--text);
    text-align: left;
  }
  .name {
    font-weight: 500;
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
