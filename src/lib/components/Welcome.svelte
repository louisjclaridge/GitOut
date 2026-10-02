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
      <img src="/logo.svg" alt="" width="44" height="44" />
      <div>
        <h1>GitOut</h1>
        <p class="muted">A small, fast git client.</p>
      </div>
    </div>

    <div class="actions">
      <button class="card" onclick={pickAndOpen}>
        <Icon name="folder" size={22} />
        <strong>Open</strong>
        <span class="muted">a repository on this computer</span>
      </button>
      <button class="card" onclick={() => (cloning = true)}>
        <Icon name="download" size={22} />
        <strong>Clone</strong>
        <span class="muted">from GitHub, GitLab, Bitbucket or a URL</span>
      </button>
      <button class="card" onclick={pickAndInit}>
        <Icon name="plus" size={22} />
        <strong>Create</strong>
        <span class="muted">a new repository</span>
      </button>
    </div>

    <div class="columns">
      <section>
        <div class="section-title">Recent</div>
        {#if app.settings?.recentRepos.length}
          <ul class="recent">
            {#each app.settings.recentRepos as r (r)}
              <li>
                <button class="row" onclick={() => openRepo(r)} title={r}>
                  <Icon name="folder" size={15} />
                  <span class="name">{basename(r)}</span>
                  <span class="path muted ellipsis">{r}</span>
                </button>
                <button class="btn ghost icon forget" title="Remove from list" onclick={() => forget(r)}>
                  <Icon name="x" size={12} />
                </button>
              </li>
            {/each}
          </ul>
        {:else}
          <p class="muted">Repositories you open will show up here.</p>
        {/if}
      </section>

      <section class="accounts">
        <div class="section-title">Accounts</div>
        {#if app.settings?.accounts.length}
          {#each app.settings.accounts as a (a.id)}
            <div class="acct">
              <ProviderLogo provider={a.provider} />
              <span class="ellipsis">{a.displayName || a.username}</span>
              <span class="muted ellipsis">{a.host}</span>
            </div>
          {/each}
        {:else}
          <p class="muted">Sign in to clone your repositories and push over HTTPS without passwords.</p>
        {/if}
        <button class="btn small" onclick={() => (app.view = "settings")}>
          <Icon name="user" size={13} /> Manage accounts
        </button>
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
    padding: 48px 24px;
  }
  .inner {
    width: min(860px, 100%);
    display: flex;
    flex-direction: column;
    gap: 32px;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 14px;
  }
  h1 {
    margin: 0;
    font-size: 24px;
    letter-spacing: -0.01em;
  }
  .brand p {
    margin: 0;
  }
  .actions {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: 12px;
  }
  .card {
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 6px;
    padding: 18px;
    text-align: left;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
    transition:
      border-color 0.12s,
      transform 0.12s;
  }
  .card:hover {
    border-color: var(--accent);
    transform: translateY(-1px);
  }
  .card :global(svg) {
    color: var(--accent);
    margin-bottom: 4px;
  }
  .card strong {
    font-size: 15px;
  }
  .columns {
    display: grid;
    grid-template-columns: 2fr 1fr;
    gap: 28px;
  }
  @media (max-width: 760px) {
    .columns {
      grid-template-columns: 1fr;
    }
  }
  section {
    display: flex;
    flex-direction: column;
    gap: 10px;
    min-width: 0;
  }
  section p {
    margin: 0;
  }
  .recent {
    list-style: none;
    margin: 0;
    padding: 0;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }
  .recent li {
    display: flex;
    align-items: center;
    border-radius: 6px;
  }
  .recent li:hover {
    background: var(--hover);
  }
  .row {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 10px;
    min-width: 0;
    padding: 7px 10px;
    border: 0;
    background: none;
    text-align: left;
  }
  .name {
    font-weight: 500;
    flex-shrink: 0;
  }
  .path {
    font-size: 12px;
  }
  .forget {
    opacity: 0;
    margin-right: 4px;
  }
  .recent li:hover .forget {
    opacity: 0.7;
  }
  .acct {
    display: flex;
    align-items: center;
    gap: 8px;
    min-width: 0;
  }
  .accounts .btn {
    align-self: flex-start;
  }
</style>
