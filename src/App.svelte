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
  <header class="topbar">
    <button
      class="k-icon-btn"
      aria-pressed={app.view === "welcome"}
      aria-label="Home"
      title="Home (Ctrl+T)"
      onclick={() => (app.view = "welcome")}
    >
      <Icon name="home" />
    </button>
    {#if app.tabs.length}
      <div class="k-tabs sessions session-tabs" role="tablist" aria-label="Open repositories">
        {#each app.tabs as t (t.path)}
          {@const selected = app.view === "repo" && app.active === t.path}
          <div
            class="k-tab"
            title={t.path}
            role="tab"
            tabindex="0"
            aria-selected={selected}
            onclick={() => selectTab(t.path)}
            onkeydown={(e) => e.key === "Enter" && selectTab(t.path)}
            onauxclick={(e) => e.button === 1 && closeTab(t.path)}
          >
            <Icon name="folder" size={14} />
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
      </div>
    {/if}
    <button class="k-icon-btn" title="New tab (Ctrl+T)" aria-label="New tab" onclick={() => (app.view = "welcome")}>
      <Icon name="plus" />
    </button>
    <span class="spacer"></span>
    <button
      class="k-icon-btn"
      aria-pressed={app.view === "settings"}
      aria-label="Settings"
      title="Settings and accounts"
      onclick={() => (app.view = "settings")}
    >
      <Icon name="settings" />
    </button>
  </header>

  {#if app.update}
    <div class="update">
      <div class="k-banner">
        <Icon name="download" size={20} />
        <div class="body">GitOut {app.update.version} is available.</div>
        <button class="k-btn ghost" onclick={() => (app.update = null)}>Later</button>
        <button
          class="k-btn primary"
          disabled={installing}
          onclick={async () => {
            installing = true;
            try {
              await app.update?.install();
            } catch (e) {
              toast(errorText(e), "error");
              installing = false;
            }
          }}>{#if installing}<span class="k-spinner"></span> Installing…{:else}Restart and update{/if}</button
        >
      </div>
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
      <span class="k-spinner" role="status" aria-label={app.busy}></span>
      <span>{app.busy}</span>
      {#if app.progress}<span class="progress mono ellipsis">{app.progress}</span>{/if}
    {:else}
      <span class="progress">{app.gitVersion ?? ""}</span>
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
  .topbar {
    flex-shrink: 0;
    display: flex;
    align-items: center;
    gap: var(--space-4);
    height: var(--titlebar-height);
    padding: 0 var(--space-8);
    background: var(--surface);
    border-bottom: 1px solid var(--border);
    z-index: var(--layer-titlebar);
  }
  .session-tabs {
    min-width: 0;
    flex: 0 1 auto;
  }
  .session-tabs .k-tab {
    min-width: 0;
    padding-right: var(--space-6);
  }
  .session-tabs .close {
    padding: 0;
    border: 0;
    background: none;
    margin-right: 0;
    flex: none;
  }
  .spacer {
    flex: 1;
  }
  .update {
    flex-shrink: 0;
    padding: var(--space-8) var(--space-8) 0;
    background: var(--bg);
  }
  .update .k-banner {
    padding: var(--space-6) var(--space-6) var(--space-6) var(--space-12);
  }
  main {
    flex: 1;
    min-height: 0;
    overflow: hidden;
  }
  .status {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    height: 24px;
    padding: 0 var(--space-12);
    font-size: 12px;
    line-height: 16px;
    color: var(--text-2);
    background: var(--bg);
    border-top: 1px solid var(--border);
    min-width: 0;
  }
  .status .k-spinner {
    width: 12px;
    height: 12px;
  }
  .progress {
    color: var(--text-3);
    min-width: 0;
  }
</style>
