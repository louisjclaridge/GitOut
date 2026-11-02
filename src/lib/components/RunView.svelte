<script lang="ts">
  import { onMount } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type CiJob } from "../api";
  import type { RepoStore } from "../repo.svelte";
  import { showMenu } from "../menu.svelte";
  import { errorText, run } from "../state.svelte";
  import { formatDuration, fullDate, isoToUnix, relativeTime, shortHash } from "../util";
  import Icon from "./Icon.svelte";
  import RunIcon, { runState } from "./RunIcon.svelte";

  let { store, id }: { store: RepoStore; id: string } = $props();

  let wf = $derived(store.runs.find((r) => r.id === id));
  let st = $derived(wf ? runState(wf.status, wf.conclusion) : null);
  let jobs = $state<CiJob[] | null>(null);
  let error = $state("");
  /** Ticks every second so running durations count up. */
  let now = $state(Date.now() / 1000);

  let done = $derived(jobs?.filter((j) => j.status === "completed").length ?? 0);
  let active = $derived(!wf || wf.status !== "completed" || !!jobs?.some((j) => j.status !== "completed"));
  let commit = $derived(wf ? store.commits.find((c) => c.hash === wf.headSha) : undefined);

  async function loadJobs() {
    try {
      jobs = await api.ciJobs(store.path, id);
      error = "";
    } catch (e) {
      error = errorText(e);
    }
  }

  // Poll the jobs (and the run list, for the run's own status) while anything is still going.
  onMount(() => {
    void loadJobs();
    const tick = setInterval(() => (now = Date.now() / 1000), 1000);
    let poll: ReturnType<typeof setTimeout>;
    const again = () => {
      poll = setTimeout(async () => {
        if (active) await Promise.all([loadJobs(), store.loadRuns()]);
        again();
      }, 4000);
    };
    again();
    return () => {
      clearInterval(tick);
      clearTimeout(poll);
    };
  });

  function duration(start: string | null, end: string | null): string {
    const a = isoToUnix(start);
    if (a === null) return "";
    return formatDuration((isoToUnix(end) ?? now) - a);
  }

  function stepsDone(j: CiJob) {
    return j.steps.filter((s) => s.status === "completed").length;
  }
</script>

<div class="run">
  <header>
    <button class="k-icon-btn" aria-label="Back to history" title="Back to history (Esc)" onclick={() => (store.center = { kind: "graph" })}>
      <Icon name="back" />
    </button>
    {#if wf && st}
      <RunIcon status={wf.status} conclusion={wf.conclusion} size={20} />
      <div class="title">
        <strong class="ellipsis selectable">{wf.title || wf.name}</strong>
        <span class="ellipsis">{wf.name} #{wf.number}{wf.attempt > 1 ? ` (attempt ${wf.attempt})` : ""}</span>
      </div>
      <span class="k-badge {st.tone === 'success' ? 'success' : st.tone === 'failure' ? 'danger' : st.tone === 'running' ? 'accent' : st.tone === 'queued' ? 'warning' : ''}">
        {st.label}
      </span>
      {#if wf.url}
        <button class="k-btn" onclick={() => wf?.url && run(() => openUrl(wf.url!))}><Icon name="external" /> Open in browser</button>
      {/if}
      <button class="k-icon-btn" aria-label="More actions" title="More actions" onclick={(e) => wf && showMenu(e, store.runMenu(wf))}>
        <Icon name="more" />
      </button>
    {:else}
      <div class="title"><strong>CI/CD run</strong></div>
    {/if}
  </header>

  <div class="content">
    {#if !wf}
      <div class="empty-note">
        {#if store.runsStatus === "loading"}
          <span class="k-spinner" role="status" aria-label="Loading run"></span>
        {:else}
          This run is no longer in the recent list.
        {/if}
      </div>
    {:else}
      <div class="meta">
        <button
          class="k-btn ghost mono"
          title={commit ? "Show this commit in the history" : `${shortHash(wf.headSha)} isn't on this computer yet. Fetch to get it.`}
          disabled={!commit}
          onclick={() => store.selectCommit(wf!.headSha)}
        >
          <Icon name="commit" />
          {shortHash(wf.headSha)}
        </button>
        {#if wf.branch}<span class="k-badge"><Icon name="branch" size={12} />{wf.branch}</span>{/if}
        <span class="when" title={wf.createdAt ? fullDate(isoToUnix(wf.createdAt)!) : undefined}>
          {wf.trigger}
          {#if wf.actor}by {wf.actor}{/if}
          {#if isoToUnix(wf.createdAt)}· {relativeTime(isoToUnix(wf.createdAt)!)}{/if}
          {#if wf.startedAt}· {wf.status === "completed" ? "took" : "running for"} {duration(wf.startedAt, wf.status === "completed" ? wf.updatedAt : null)}{/if}
        </span>
      </div>

      {#if error && !jobs}
        <div class="k-banner danger" role="alert">
          <Icon name="error" />
          <div class="body selectable">{error}</div>
          <button class="k-btn" onclick={loadJobs}>Try again</button>
        </div>
      {:else if !jobs}
        <div class="empty-note"><span class="k-spinner" role="status" aria-label="Loading jobs"></span></div>
      {:else if !jobs.length}
        <p class="k-muted">{wf.status === "completed" ? "This run had no jobs." : "Waiting for jobs to start…"}</p>
      {:else}
        <div>
          <div class="k-progress-label">
            <b>Jobs</b>
            <span>{done} of {jobs.length} finished</span>
          </div>
          <div class="k-progress stacked" role="progressbar" aria-valuemin={0} aria-valuemax={jobs.length} aria-valuenow={done}>
            {#each jobs as j (j.id)}
              {@const t = runState(j.status, j.conclusion).tone}
              <span
                class={t === "success" ? "success" : t === "failure" ? "danger" : t === "running" ? "" : "pending"}
                style="width:{100 / jobs.length}%"
              ></span>
            {/each}
          </div>
        </div>

        <div class="k-cards">
          {#each jobs as j (j.id)}
            {@const total = j.steps.length}
            {@const finished = stepsDone(j)}
            <section class="k-card flush job">
              <div class="section head">
                <RunIcon status={j.status} conclusion={j.conclusion} />
                <strong class="ellipsis selectable">{j.name}</strong>
                <span class="dur">{duration(j.startedAt, j.completedAt)}</span>
                {#if j.url}
                  <button class="k-icon-btn" aria-label="Open {j.name} in browser" title="Open logs in browser" onclick={() => run(() => openUrl(j.url!))}>
                    <Icon name="external" size={14} />
                  </button>
                {/if}
              </div>
              {#if total}
                {#if j.status !== "completed"}
                  <div class="k-progress bar"><span style="width:{(finished / total) * 100}%"></span></div>
                {/if}
                <ol class="steps">
                  {#each j.steps as s (s.number)}
                    <li class:current={s.status === "running"}>
                      <RunIcon status={s.status} conclusion={s.conclusion} size={14} />
                      <span class="name ellipsis">{s.name}</span>
                      <span class="dur">{s.status === "queued" || s.conclusion === "skipped" ? "" : duration(s.startedAt, s.completedAt)}</span>
                    </li>
                  {/each}
                </ol>
              {/if}
            </section>
          {/each}
        </div>
      {/if}
    {/if}
  </div>
</div>

<style>
  .run {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  header {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    min-height: 48px;
    padding: var(--space-6) var(--space-8);
    border-bottom: 1px solid var(--border);
  }
  .title {
    flex: 1;
    min-width: 0;
    display: flex;
    flex-direction: column;
  }
  .title strong {
    font-family: var(--font-display);
    font-weight: 700;
  }
  .title span {
    font-size: 11px;
    line-height: 16px;
    color: var(--text-3);
  }
  .content {
    flex: 1;
    overflow: auto;
    padding: var(--space-16) var(--space-24) var(--space-32);
    display: flex;
    flex-direction: column;
    gap: var(--space-16);
    max-width: 820px;
  }
  .meta {
    display: flex;
    align-items: center;
    flex-wrap: wrap;
    gap: var(--space-8);
    margin-left: calc(-1 * var(--space-12));
  }
  .when {
    color: var(--text-3);
    font-size: 12px;
  }
  .k-progress > .pending {
    background: transparent;
  }
  .k-progress > span:not(.success, .danger, .pending) {
    background: var(--accent);
    opacity: 0.6;
  }
  .k-progress.stacked > span + span {
    box-shadow: inset 2px 0 0 var(--surface);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    min-height: 44px;
    padding-top: var(--space-4);
    padding-bottom: var(--space-4);
  }
  .head strong {
    flex: 1;
    min-width: 0;
    font-weight: 600;
  }
  .dur {
    flex: none;
    font-size: 12px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .bar {
    height: 3px;
    border-radius: 0;
  }
  .bar > span {
    border-radius: 0;
  }
  .steps {
    margin: 0;
    padding: var(--space-6) 0;
    list-style: none;
    border-top: 1px solid var(--border);
  }
  .steps li {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    min-height: 28px;
    padding: 0 var(--space-16);
    color: var(--text-2);
  }
  .steps li.current {
    color: var(--text);
    font-weight: 600;
    background: color-mix(in srgb, var(--accent-soft) 60%, transparent);
  }
  .steps .name {
    flex: 1;
    min-width: 0;
  }
  .empty-note .k-spinner {
    display: inline-block;
  }
</style>
