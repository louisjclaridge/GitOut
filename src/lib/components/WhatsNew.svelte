<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { run } from "../state.svelte";
  import { releaseNotes, renderNotes, whatsNew } from "../whatsnew.svelte";
  import Icon from "./Icon.svelte";

  // Dev builds have no baked-in notes; open the dev server with ?whatsnew to preview with these.
  const SAMPLE = `## What's new\n\n- **CI/CD** in the sidebar: watch runs, jobs and steps as they happen.\n- **Fetch & pull all** brings every open repository up to date.\n- Drag the sidebar edges to resize them.\n\n### Fixes\n\n- Long branch names no longer stretch the filter box. See https://example.com/notes`;

  let html = $derived(renderNotes(releaseNotes() || (import.meta.env.DEV ? releaseNotes(SAMPLE) : "")));

  function close() {
    whatsNew.open = false;
  }

  // Links open in the browser, not inside the app window.
  function onclick(e: MouseEvent) {
    const a = (e.target as HTMLElement).closest("a");
    if (!a) return;
    e.preventDefault();
    const href = a.getAttribute("href");
    if (href) void run(() => openUrl(href));
  }
</script>

<svelte:window onkeydown={(e) => whatsNew.open && e.key === "Escape" && close()} />

{#if whatsNew.open}
  <div class="k-scrim" role="presentation" onmousedown={(e) => e.target === e.currentTarget && close()}>
    <div class="k-dialog whats-new" role="dialog" aria-modal="true" aria-labelledby="whats-new-title">
      <button class="k-icon-btn close" aria-label="Close" onclick={close}><Icon name="x" size={14} /></button>
      <div class="head">
        <span class="k-tile lg"><Icon name="release" size={20} /></span>
        <div>
          <h2 id="whats-new-title">What's new</h2>
          {#if whatsNew.version}<span class="k-muted">GitOut {whatsNew.version}</span>{/if}
        </div>
      </div>
      <!-- svelte-ignore a11y_click_events_have_key_events, a11y_no_static_element_interactions -->
      <div class="notes selectable" {onclick}>
        {#if html}
          {@html html}
        {:else}
          <p>No notes for this version.</p>
        {/if}
      </div>
      <div class="actions">
        <button class="k-btn primary" onclick={close}>Got it</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .whats-new {
    width: min(540px, 100%);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-12);
  }
  .head h2 {
    padding-right: 0;
    font-size: 22px;
    line-height: 28px;
    font-weight: 800;
    letter-spacing: -0.02em;
  }
  .notes {
    max-height: min(420px, 60vh);
    overflow: auto;
    padding: var(--space-4) var(--space-16);
    border-radius: var(--radius-lg);
    background: var(--surface);
    box-shadow: var(--shadow-card);
    color: var(--text-2);
  }
  .notes :global(h3) {
    margin: var(--space-12) 0 var(--space-4);
    font-family: var(--font-display);
    font-size: 15px;
    line-height: 20px;
    font-weight: 700;
    color: var(--text);
  }
  .notes :global(p) {
    margin: var(--space-8) 0;
    color: var(--text-2);
  }
  .notes :global(ul) {
    margin: var(--space-8) 0;
    padding-left: var(--space-20);
  }
  .notes :global(li + li) {
    margin-top: var(--space-4);
  }
  .notes :global(strong) {
    color: var(--text);
  }
  .notes :global(code) {
    padding: 0 4px;
    border-radius: var(--radius-xs);
    background: var(--surface-2);
  }
  .notes :global(a) {
    color: var(--accent-text);
    word-break: break-all;
  }
</style>
