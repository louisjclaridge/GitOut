<script lang="ts">
  import { onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { homeDir, join } from "@tauri-apps/api/path";
  import { api, type RemoteRepo } from "../api";
  import { app, addTab, run, errorText } from "../state.svelte";
  import { repoNameFromUrl } from "../util";
  import Icon from "./Icon.svelte";
  import ProviderLogo from "./ProviderLogo.svelte";

  let { onclose }: { onclose: () => void } = $props();

  const PARENT_KEY = "gitout.cloneParent";

  let source = $state<string>("url"); // "url" or an account id
  let url = $state("");
  let parent = $state("");
  let name = $state("");
  let nameTouched = $state(false);
  let repos = $state<RemoteRepo[]>([]);
  let loadingRepos = $state(false);
  let repoError = $state("");
  let filter = $state("");
  let useSsh = $state(false);

  onMount(async () => {
    parent = localStorage.getItem(PARENT_KEY) ?? (await join(await homeDir(), "Projects"));
    const first = app.settings?.accounts[0];
    if (first) selectSource(first.id);
  });

  $effect(() => {
    if (!nameTouched) name = repoNameFromUrl(url);
  });

  async function selectSource(id: string) {
    source = id;
    if (id === "url") return;
    loadingRepos = true;
    repoError = "";
    repos = [];
    try {
      repos = await api.authListRepos(id);
    } catch (e) {
      repoError = errorText(e);
    } finally {
      loadingRepos = false;
    }
  }

  function pick(r: RemoteRepo) {
    url = useSsh && r.sshUrl ? r.sshUrl : r.httpsUrl;
    nameTouched = false;
  }

  let filtered = $derived(
    repos.filter((r) => !filter || r.fullName.toLowerCase().includes(filter.toLowerCase())),
  );

  async function browse() {
    const dir = await open({ directory: true, title: "Clone into…", defaultPath: parent });
    if (typeof dir === "string") parent = dir;
  }

  async function clone(e: Event) {
    e.preventDefault();
    if (!url.trim() || !parent.trim() || !name.trim()) return;
    localStorage.setItem(PARENT_KEY, parent);
    onclose();
    const info = await run(() => api.repoClone(url.trim(), parent.trim(), name.trim()), {
      label: `Cloning ${name}`,
      done: `Cloned ${name}`,
    });
    if (info) addTab(info);
  }
</script>

<svelte:window onkeydown={(e) => e.key === "Escape" && onclose()} />

<div class="k-scrim" role="presentation" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <div class="k-dialog clone" role="dialog" aria-modal="true" aria-labelledby="clone-title">
    <form onsubmit={clone}>
      <header>
        <h2 id="clone-title">Clone a repository</h2>
        <button type="button" class="k-icon-btn" aria-label="Close" title="Close" onclick={onclose}><Icon name="x" /></button>
      </header>

      <div class="cols">
        <nav class="sources k-list" aria-label="Source">
          <button type="button" class="k-item" aria-current={source === "url" ? "true" : undefined} onclick={() => selectSource("url")}>
            <Icon name="external" />
            <span class="name">URL</span>
          </button>
          {#each app.settings?.accounts ?? [] as a (a.id)}
            <button type="button" class="k-item" aria-current={source === a.id ? "true" : undefined} onclick={() => selectSource(a.id)}>
              <ProviderLogo provider={a.provider} />
              <span class="name">{a.username}</span>
            </button>
          {/each}
          {#if !app.settings?.accounts.length}
            <button
              type="button"
              class="k-link signin"
              onclick={() => {
                onclose();
                app.view = "settings";
              }}>Sign in to see your repositories…</button
            >
          {/if}
        </nav>

        <div class="main">
          {#if source !== "url"}
            <div class="list-head">
              <div class="k-search">
                <Icon name="search" size={14} />
                <input class="k-input" placeholder="Filter repositories" aria-label="Filter repositories" bind:value={filter} spellcheck="false" />
              </div>
              <label class="k-check"><input type="checkbox" bind:checked={useSsh} /><span>Use SSH</span></label>
            </div>
            <div class="list">
              {#if loadingRepos}
                <div class="empty-note loading"><span class="k-spinner"></span> Loading repositories…</div>
              {:else if repoError}
                <div class="k-banner danger" role="alert">
                  <Icon name="error" size={20} />
                  <div class="body"><div class="desc selectable">{repoError}</div></div>
                </div>
              {:else if !filtered.length}
                <div class="empty-note">{filter ? `No repositories match “${filter}”.` : "No repositories found."}</div>
              {/if}
              {#each filtered as r (r.fullName)}
                <button
                  type="button"
                  class="repo"
                  class:selected={url === r.httpsUrl || url === r.sshUrl}
                  onclick={() => pick(r)}
                  ondblclick={(e) => {
                    pick(r);
                    void clone(e);
                  }}
                >
                  <span class="repo-name ellipsis">{r.fullName}</span>
                  {#if r.private}<span class="k-badge">Private</span>{/if}
                  {#if r.description}<span class="repo-desc ellipsis">{r.description}</span>{/if}
                </button>
              {/each}
            </div>
          {/if}

          <label class="k-field">
            <span class="label">Repository URL</span>
            <input class="k-input mono" bind:value={url} placeholder="https://github.com/owner/repo.git" spellcheck="false" />
          </label>
          <div class="row">
            <label class="k-field grow">
              <span class="label">Clone into</span>
              <input class="k-input mono" bind:value={parent} spellcheck="false" />
            </label>
            <button type="button" class="k-btn" onclick={browse}>Browse…</button>
          </div>
          <label class="k-field">
            <span class="label">Folder name</span>
            <input class="k-input mono" bind:value={name} oninput={() => (nameTouched = true)} spellcheck="false" />
          </label>
        </div>
      </div>

      <footer class="actions">
        <button type="button" class="k-btn ghost" onclick={onclose}>Cancel</button>
        <button type="submit" class="k-btn primary" disabled={!url.trim() || !parent.trim() || !name.trim()}>Clone</button>
      </footer>
    </form>
  </div>
</div>

<style>
  /* A two-column form, so wider than Keel's default dialog. */
  .clone {
    width: min(760px, 100%);
    gap: 0;
    padding: 0;
    overflow: hidden;
  }
  form {
    display: contents;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: var(--space-12) var(--space-12) var(--space-12) var(--space-20);
    border-bottom: 1px solid var(--border);
  }
  .cols {
    display: grid;
    grid-template-columns: 188px 1fr;
    min-height: 0;
    flex: 1;
  }
  .sources {
    padding: var(--space-12) var(--space-8);
    background: var(--bg);
    border-right: 1px solid var(--border);
  }
  .signin {
    padding: var(--space-6) var(--space-8);
    font-size: 12px;
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: var(--space-12);
    padding: var(--space-16) var(--space-20);
    min-width: 0;
    overflow: auto;
  }
  .list-head {
    display: flex;
    gap: var(--space-12);
    align-items: center;
  }
  .list-head .k-search {
    flex: 1;
  }
  .k-check {
    white-space: nowrap;
  }
  .list {
    height: 230px;
    overflow: auto;
    padding: var(--space-4);
    border-radius: var(--radius-md);
    box-shadow: inset 0 0 0 1px var(--border);
  }
  .loading {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-8);
  }
  .repo {
    display: grid;
    grid-template-columns: 1fr auto;
    align-items: center;
    gap: 0 var(--space-8);
    width: 100%;
    min-height: var(--control-md);
    padding: var(--space-6) var(--space-8);
    border: 0;
    border-radius: var(--radius-sm);
    background: none;
    color: var(--text);
    text-align: left;
  }
  .repo:hover {
    background: var(--surface-hover);
  }
  .repo.selected {
    background: var(--accent-soft);
  }
  .repo-name {
    font-weight: 600;
  }
  .repo-desc {
    grid-column: 1 / -1;
    font-size: 12px;
    line-height: 16px;
    color: var(--text-3);
  }
  .row {
    display: flex;
    gap: var(--space-8);
    align-items: flex-end;
  }
  .grow {
    flex: 1;
  }
  .clone .actions {
    margin: 0;
    padding: var(--space-12) var(--space-20);
    border-top: 1px solid var(--border);
  }
</style>
