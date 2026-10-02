<script lang="ts">
  import { onMount } from "svelte";
  import { listen } from "@tauri-apps/api/event";
  import { api } from "./lib/api";
  import {
    app,
    applyTheme,
    closeTab,
    loadSettings,
    restoreTabs,
    selectTab,
    toast,
    errorText,
  } from "./lib/state.svelte";
  import { checkForUpdate } from "./lib/updater";
  import Icon from "./lib/components/Icon.svelte";
  import Welcome from "./lib/components/Welcome.svelte";
  import RepoView from "./lib/components/RepoView.svelte";
  import SettingsView from "./lib/components/Settings.svelte";
  import Toasts from "./lib/components/Toasts.svelte";
  import Dialog from "./lib/components/Dialog.svelte";
  import ContextMenu from "./lib/components/ContextMenu.svelte";

  onMount(() => {
    const unlisten = listen<{ repo: string; line: string }>("git-progress", (e) => {
      app.progress = e.payload.line;
    });
    const mq = matchMedia("(prefers-color-scheme: dark)");
    mq.addEventListener("change", applyTheme);

    (async () => {
      try {
        await loadSettings();
        app.gitVersion = await api.gitVersion();
      } catch (e) {
        toast(errorText(e), "error");
      }
      await restoreTabs();
      if (app.settings?.autoUpdate) void checkForUpdate(false);
    })();

    return () => {
      void unlisten.then((f) => f());
      mq.removeEventListener("change", applyTheme);
    };
  });

  let installing = $state(false);
</script>

<svelte:window
  onkeydown={(e) => {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "t") {
      e.preventDefault();
      app.view = "welcome";
    }
  }}
/>

<div class="shell">
  <header class="tabs">
    <button
      class="tab home"
      class:active={app.view === "welcome"}
      title="Home"
      onclick={() => (app.view = "welcome")}
    >
      <Icon name="home" size={15} />
    </button>
    <div class="tab-strip">
      {#each app.tabs as t (t.path)}
        <div
          class="tab"
          class:active={app.view === "repo" && app.active === t.path}
          title={t.path}
          role="tab"
          tabindex="0"
          aria-selected={app.active === t.path}
          onclick={() => selectTab(t.path)}
          onkeydown={(e) => e.key === "Enter" && selectTab(t.path)}
          onauxclick={(e) => e.button === 1 && closeTab(t.path)}
        >
          <Icon name="folder" size={13} />
          <span class="ellipsis">{t.name}</span>
          <button
            class="close"
            aria-label="Close {t.name}"
            onclick={(e) => {
              e.stopPropagation();
              closeTab(t.path);
            }}><Icon name="x" size={12} /></button
          >
        </div>
      {/each}
      <button class="new-tab" title="New tab (Ctrl+T)" aria-label="New tab" onclick={() => (app.view = "welcome")}>
        <Icon name="plus" size={14} />
      </button>
    </div>
    <button
      class="tab home"
      class:active={app.view === "settings"}
      title="Settings & accounts"
      onclick={() => (app.view = "settings")}
    >
      <Icon name="settings" size={15} />
    </button>
  </header>

  {#if app.update}
    <div class="update-bar">
      <Icon name="download" size={14} />
      <span>GitOut {app.update.version} is available.</span>
      <button
        class="btn small primary"
        disabled={installing}
        onclick={async () => {
          installing = true;
          try {
            await app.update?.install();
          } catch (e) {
            toast(errorText(e), "error");
            installing = false;
          }
        }}>{installing ? "Installing…" : "Restart & update"}</button
      >
      <button class="btn small ghost" onclick={() => (app.update = null)}>Later</button>
    </div>
  {/if}

  <main>
    {#if app.view === "settings"}
      <SettingsView />
    {:else if app.view === "repo" && app.active}
      {#key app.active}
        <RepoView repo={app.active} />
      {/key}
    {:else}
      <Welcome />
    {/if}
  </main>

  <footer class="status">
    {#if app.busy}
      <span class="spinner"></span>
      <span>{app.busy}</span>
      {#if app.progress}<span class="muted ellipsis">— {app.progress}</span>{/if}
    {:else}
      <span class="muted">{app.gitVersion ?? ""}</span>
    {/if}
  </footer>
</div>

<Toasts />
<Dialog />
<ContextMenu />

<style>
  .shell {
    display: flex;
    flex-direction: column;
    height: 100%;
  }
  .tabs {
    flex-shrink: 0;
    display: flex;
    align-items: stretch;
    height: 38px;
    background: var(--panel);
    border-bottom: 1px solid var(--border);
  }
  .tab-strip {
    flex: 1;
    display: flex;
    overflow-x: auto;
    scrollbar-width: none;
  }
  .tab {
    display: flex;
    align-items: center;
    gap: 7px;
    min-width: 0;
    max-width: 220px;
    padding: 0 8px 0 12px;
    border: 0;
    border-right: 1px solid var(--border);
    background: none;
    color: var(--muted);
    position: relative;
  }
  .tab:hover {
    background: var(--hover);
    color: var(--text);
  }
  .tab.active {
    background: var(--bg);
    color: var(--text);
  }
  .tab.active::after {
    content: "";
    position: absolute;
    left: 0;
    right: 0;
    top: 0;
    height: 2px;
    background: var(--accent);
  }
  .tab.home {
    padding: 0 13px;
  }
  .tab.home:last-child {
    border-right: 0;
    border-left: 1px solid var(--border);
  }
  .new-tab {
    display: flex;
    align-items: center;
    align-self: center;
    flex-shrink: 0;
    margin: 0 6px;
    padding: 5px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--muted);
  }
  .new-tab:hover {
    background: var(--hover);
    color: var(--text);
  }
  .close {
    display: inline-flex;
    padding: 3px;
    border: 0;
    border-radius: 4px;
    background: none;
    color: inherit;
    opacity: 0.5;
  }
  .close:hover {
    opacity: 1;
    background: var(--hover);
  }
  .update-bar {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 6px 14px;
    background: var(--accent-soft);
    border-bottom: 1px solid var(--border);
  }
  .update-bar span {
    flex: 1;
  }
  main {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 8px;
    height: 24px;
    padding: 0 12px;
    font-size: 11.5px;
    background: var(--panel);
    border-top: 1px solid var(--border);
    min-width: 0;
  }
  .spinner {
    width: 11px;
    height: 11px;
    border: 2px solid var(--accent);
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
    flex-shrink: 0;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
</style>
