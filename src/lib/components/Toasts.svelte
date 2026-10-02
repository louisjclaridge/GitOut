<script lang="ts">
  import { app, dismissToast } from "../state.svelte";
  import Icon from "./Icon.svelte";
</script>

<div class="toasts">
  {#each app.toasts as t (t.id)}
    <div class="toast {t.kind}" role="status">
      <Icon name={t.kind === "error" ? "warning" : t.kind === "success" ? "check" : "commit"} size={15} />
      <div class="text selectable">{t.text}</div>
      <button class="btn ghost icon" aria-label="Dismiss" onclick={() => dismissToast(t.id)}>
        <Icon name="x" size={13} />
      </button>
    </div>
  {/each}
</div>

<style>
  .toasts {
    position: fixed;
    right: 16px;
    bottom: 36px;
    z-index: 950;
    display: flex;
    flex-direction: column;
    gap: 8px;
    width: min(420px, calc(100vw - 32px));
  }
  .toast {
    display: flex;
    align-items: flex-start;
    gap: 10px;
    padding: 10px 8px 10px 12px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-left: 3px solid var(--accent);
    border-radius: 8px;
    box-shadow: var(--shadow);
    animation: in 0.15s ease-out;
  }
  .toast.error {
    border-left-color: var(--danger);
    color: var(--text);
  }
  .toast.error :global(svg) {
    color: var(--danger);
  }
  .toast.success {
    border-left-color: var(--ok);
  }
  .toast.success :global(svg) {
    color: var(--ok);
  }
  .text {
    flex: 1;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow: auto;
    padding-top: 1px;
  }
  @keyframes in {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
</style>
