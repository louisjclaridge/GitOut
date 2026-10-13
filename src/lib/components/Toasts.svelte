<script lang="ts">
  import { app, dismissToast } from "../state.svelte";
  import Icon from "./Icon.svelte";

  const TONE = { info: "", success: "success", error: "danger" } as const;
</script>

<div class="k-toasts">
  {#each app.toasts.slice(-3) as t (t.id)}
    <div class="k-toast {TONE[t.kind]}" role={t.kind === "error" ? "alert" : "status"}>
      <Icon name={t.kind} size={20} />
      <div class="body selectable">{t.text}</div>
      <button class="k-icon-btn" aria-label="Dismiss" title="Dismiss" onclick={() => dismissToast(t.id)}>
        <Icon name="x" />
      </button>
    </div>
  {/each}
</div>

<style>
  /* Sit above the status bar. */
  .k-toasts {
    bottom: 40px;
  }
  .k-toast {
    align-items: flex-start;
    padding: var(--space-6) var(--space-6) var(--space-6) var(--space-12);
    animation: enter 150ms cubic-bezier(0.2, 0.8, 0.2, 1);
  }
  .k-toast > :global(.k-icon) {
    margin-top: 6px;
  }
  .body {
    padding: 6px 0;
    white-space: pre-wrap;
    word-break: break-word;
    max-height: 200px;
    overflow: auto;
  }
  @keyframes enter {
    from {
      transform: translateY(8px);
      opacity: 0;
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .k-toast {
      animation: none;
    }
  }
</style>
