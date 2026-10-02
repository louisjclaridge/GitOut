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

<div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && onclose()}>
  <form class="panel" onsubmit={clone}>
    <header>
      <h3>Clone a repository</h3>
      <button type="button" class="btn ghost icon" aria-label="Close" onclick={onclose}><Icon name="x" /></button>
    </header>

    <div class="body">
      <nav class="sources">
        <button type="button" class:active={source === "url"} onclick={() => selectSource("url")}>
          <Icon name="external" size={14} /> URL
        </button>
        {#each app.settings?.accounts ?? [] as a (a.id)}
          <button type="button" class:active={source === a.id} onclick={() => selectSource(a.id)}>
            <ProviderLogo provider={a.provider} size={14} />
            <span class="ellipsis">{a.username}</span>
          </button>
        {/each}
        {#if !app.settings?.accounts.length}
          <button
            type="button"
            class="signin"
            onclick={() => {
              onclose();
              app.view = "settings";
            }}>+ Sign in to see your repos</button
          >
        {/if}
      </nav>

      <div class="main">
        {#if source !== "url"}
          <div class="list-head">
            <input placeholder="Filter repositories…" bind:value={filter} spellcheck="false" />
            <label class="ssh"><input type="checkbox" bind:checked={useSsh} /> SSH</label>
          </div>
          <div class="list">
            {#if loadingRepos}
              <div class="empty">Loading repositories…</div>
            {:else if repoError}
              <div class="empty error">{repoError}</div>
            {:else if !filtered.length}
              <div class="empty">No repositories found.</div>
            {/if}
            {#each filtered as r (r.fullName)}
              <button
                type="button"
                class="repo"
                class:sel={url === r.httpsUrl || url === r.sshUrl}
                onclick={() => pick(r)}
                ondblclick={(e) => {
                  pick(r);
                  void clone(e);
                }}
              >
                <span class="ellipsis"><strong>{r.fullName}</strong></span>
                {#if r.private}<span class="badge">private</span>{/if}
                {#if r.description}<span class="desc muted ellipsis">{r.description}</span>{/if}
              </button>
            {/each}
          </div>
        {/if}

        <label class="field">
          <span>Repository URL</span>
          <input bind:value={url} placeholder="https://github.com/owner/repo.git" spellcheck="false" />
        </label>
        <div class="row">
          <label class="field grow">
            <span>Clone into</span>
            <input bind:value={parent} spellcheck="false" />
          </label>
          <button type="button" class="btn" onclick={browse}>Browse…</button>
        </div>
        <label class="field">
          <span>Folder name</span>
          <input bind:value={name} oninput={() => (nameTouched = true)} spellcheck="false" />
        </label>
      </div>
    </div>

    <footer>
      <button type="button" class="btn" onclick={onclose}>Cancel</button>
      <button type="submit" class="btn primary" disabled={!url.trim() || !parent.trim() || !name.trim()}>
        Clone
      </button>
    </footer>
  </form>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 800;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.45);
  }
  .panel {
    width: min(760px, calc(100vw - 32px));
    max-height: calc(100vh - 64px);
    display: flex;
    flex-direction: column;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
  }
  header,
  footer {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 12px 16px;
  }
  header {
    border-bottom: 1px solid var(--border);
  }
  footer {
    justify-content: flex-end;
    gap: 8px;
    border-top: 1px solid var(--border);
  }
  h3 {
    margin: 0;
    font-size: 15px;
  }
  .body {
    display: grid;
    grid-template-columns: 180px 1fr;
    min-height: 0;
    flex: 1;
  }
  .sources {
    display: flex;
    flex-direction: column;
    gap: 2px;
    padding: 10px;
    border-right: 1px solid var(--border);
  }
  .sources button {
    display: flex;
    align-items: center;
    gap: 8px;
    padding: 7px 10px;
    border: 0;
    border-radius: 6px;
    background: none;
    text-align: left;
    min-width: 0;
  }
  .sources button:hover {
    background: var(--hover);
  }
  .sources button.active {
    background: var(--accent-soft);
    color: var(--accent);
  }
  .sources .signin {
    color: var(--accent);
    font-size: 12px;
  }
  .main {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 14px 16px;
    min-width: 0;
    overflow: auto;
  }
  .list-head {
    display: flex;
    gap: 10px;
    align-items: center;
  }
  .ssh {
    display: flex;
    gap: 5px;
    align-items: center;
    white-space: nowrap;
  }
  .list {
    height: 230px;
    overflow: auto;
    border: 1px solid var(--border);
    border-radius: 8px;
    background: var(--bg);
  }
  .repo {
    display: grid;
    grid-template-columns: 1fr auto;
    gap: 2px 8px;
    width: 100%;
    padding: 7px 10px;
    border: 0;
    border-bottom: 1px solid var(--border);
    background: none;
    text-align: left;
  }
  .repo:hover {
    background: var(--hover);
  }
  .repo.sel {
    background: var(--selected);
  }
  .desc {
    grid-column: 1 / -1;
    font-size: 12px;
  }
  .badge {
    font-size: 10.5px;
    padding: 0 6px;
    border-radius: 9px;
    border: 1px solid var(--border);
    color: var(--muted);
    align-self: center;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .field span {
    font-size: 12px;
    color: var(--muted);
  }
  .row {
    display: flex;
    gap: 8px;
    align-items: flex-end;
  }
  .grow {
    flex: 1;
  }
  .error {
    color: var(--danger);
  }
</style>
