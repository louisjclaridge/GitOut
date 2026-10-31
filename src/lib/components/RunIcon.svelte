<script lang="ts" module>
  export type RunTone = "running" | "queued" | "success" | "failure" | "neutral";

  /** How a CI/CD run, job or step reads at a glance. */
  export function runState(status: string, conclusion: string | null): { tone: RunTone; icon: string; label: string } {
    if (status === "running") return { tone: "running", icon: "running", label: "In progress" };
    if (status !== "completed") return { tone: "queued", icon: "queued", label: status === "waiting" ? "Waiting" : "Queued" };
    switch (conclusion) {
      case "success":
        return { tone: "success", icon: "success", label: "Succeeded" };
      case "failure":
      case "timed_out":
      case "startup_failure":
        return { tone: "failure", icon: "error", label: conclusion === "timed_out" ? "Timed out" : "Failed" };
      case "action_required":
        return { tone: "queued", icon: "warning", label: "Needs approval" };
      case "cancelled":
        return { tone: "neutral", icon: "cancelled", label: "Cancelled" };
      case "skipped":
        return { tone: "neutral", icon: "skipped", label: "Skipped" };
      default:
        return { tone: "neutral", icon: "success", label: conclusion ? conclusion[0].toUpperCase() + conclusion.slice(1) : "Completed" };
    }
  }
</script>

<script lang="ts">
  import Icon from "./Icon.svelte";

  let { status, conclusion, size = 16 }: { status: string; conclusion: string | null; size?: number } = $props();

  let st = $derived(runState(status, conclusion));
</script>

<span class="run-icon {st.tone}" title={st.label} role="img" aria-label={st.label}><Icon name={st.icon} {size} /></span>

<style>
  .run-icon {
    display: inline-flex;
    flex: none;
  }
  /* Keep the status colour inside list rows, which tint their icons. */
  .run-icon.run-icon :global(.k-icon) {
    color: inherit;
  }
  .running {
    color: var(--accent-text);
    animation: spin 1.2s linear infinite;
  }
  .queued {
    color: var(--warning-text);
  }
  .success {
    color: var(--success-text);
  }
  .failure {
    color: var(--danger-text);
  }
  .neutral {
    color: var(--text-3);
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  @media (prefers-reduced-motion: reduce) {
    .running {
      animation-duration: 3s;
    }
  }
</style>
