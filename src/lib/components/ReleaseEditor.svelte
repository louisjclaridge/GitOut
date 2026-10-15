<script lang="ts">
  import { api, type Release, type ReleaseInput } from "../api";
  import type { RepoStore } from "../repo.svelte";
  import { confirm, errorText, run, runOk, toast } from "../state.svelte";
  import { nextVersions, shortHash } from "../util";
  import Icon from "./Icon.svelte";

  let {
    store,
    id,
    tag: fromTag,
    target: atTarget,
  }: { store: RepoStore; id: string | null; tag?: string; target?: string } = $props();

  // Recreated per release ({#key} in RepoView), so the initial props are the right ones.
  // svelte-ignore state_referenced_locally
  const existing: Release | undefined = id ? store.releases.find((r) => r.id === id) : undefined;
  // svelte-ignore state_referenced_locally
  const initial = {
    tag: existing?.tag ?? fromTag ?? "",
    name: existing?.name ?? "",
    body: existing?.body ?? "",
    draft: existing?.draft ?? false,
    prerelease: existing?.prerelease ?? false,
  };
  let tag = $state(initial.tag);
  let name = $state(initial.name);
  let body = $state(initial.body);
  let draft = $state(initial.draft);
  let prerelease = $state(initial.prerelease);
  // svelte-ignore state_referenced_locally
  let target = $state(atTarget ?? store.currentBranch ?? store.headHash ?? "HEAD");
  let saving = $state(false);
  let generating = $state(false);

  let host = $derived(store.releaseHost);
  let trimmedTag = $derived(tag.trim());
  let tagRef = $derived(store.tagRef(trimmedTag));
  let taken = $derived(!existing && !!trimmedTag && !!store.releaseFor(trimmedTag));
  // Released tags count too: they may not have been fetched to this computer.
  let versions = $derived(
    existing || fromTag
      ? null
      : nextVersions([...store.refs.filter((r) => r.kind === "tag").map((r) => r.short), ...store.releases.map((r) => r.tag)]),
  );
  let dirty = $derived(
    tag !== initial.tag || name !== initial.name || body !== initial.body || draft !== initial.draft || prerelease !== initial.prerelease,
  );

  let targets = $derived.by(() => {
    const opts: { value: string; label: string }[] = [];
    if (atTarget && !store.refs.some((r) => r.kind === "local" && r.short === atTarget)) {
      opts.push({ value: atTarget, label: `Commit ${shortHash(atTarget)}` });
    }
    if (!store.currentBranch && store.headHash && store.headHash !== atTarget) {
      opts.push({ value: store.headHash, label: `HEAD (${shortHash(store.headHash)})` });
    }
    for (const r of store.refs) {
      if (r.kind === "local") opts.push({ value: r.short, label: r.current ? `${r.short} (current branch)` : r.short });
    }
    return opts;
  });

  let ready = $derived(store.releasesStatus === "loaded" && !!host && !host.problem);
  let canSave = $derived(ready && !saving && !!trimmedTag && !taken && (!existing || dirty));

  async function close() {
    if (dirty && !saving && !(await confirm("Discard changes?", "Your changes to this release haven't been saved.", "Discard", true))) return;
    store.center = existing ? { kind: "release", id: existing.id } : { kind: "graph" };
  }

  async function generate() {
    generating = true;
    try {
      const log = await api.releaseChangelog(store.path, tagRef ? trimmedTag : target);
      if (!log.lines.length) {
        toast(log.previous ? `No new commits since ${log.previous}` : "No commits to list");
        return;
      }
      const notes = (log.previous ? `Changes since ${log.previous}:\n\n` : "") + log.lines.map((l) => `- ${l}`).join("\n");
      body = body.trim() ? `${body.trimEnd()}\n\n${notes}` : notes;
    } catch (e) {
      toast(errorText(e), "error");
    } finally {
      generating = false;
    }
  }

  async function save() {
    if (!canSave) return;
    saving = true;
    try {
      const input: ReleaseInput = { tag: trimmedTag, name: name.trim(), body, draft, prerelease };
      const title = input.name || input.tag;
      let saved: Release | undefined;
      if (existing) {
        saved = await run(() => api.releaseUpdate(store.path, existing.id, input), { label: "Saving release", done: `Saved ${title}` });
      } else {
        // Releases point at a tag on the server, so make sure it exists there first.
        if (!tagRef && !(await runOk(() => api.tagCreate(store.path, input.tag, target, title), { done: `Tagged ${input.tag}` }))) return;
        if (!(await runOk(() => api.pushTag(store.path, input.tag), { label: `Pushing ${input.tag}` }))) return;
        saved = await run(() => api.releaseCreate(store.path, input), {
          label: draft ? "Saving draft" : "Publishing release",
          done: draft ? `Saved draft ${title}` : `Published ${title}`,
        });
      }
      if (!saved) return;
      await store.loadReleases();
      store.showRelease(saved.id);
    } finally {
      saving = false;
    }
  }

  let saveLabel = $derived(existing ? (existing.draft && !draft ? "Publish release" : "Save changes") : draft ? "Save draft" : "Publish release");
</script>

<div class="editor">
  <header>
    <button class="k-icon-btn" aria-label="Cancel" title="Cancel" onclick={close}><Icon name="back" /></button>
    <div class="title">
      <strong class="ellipsis">{existing ? `Edit ${existing.name || existing.tag}` : "New release"}</strong>
      {#if host && !host.problem}<span class="ellipsis">{host.project} on {host.host}</span>{/if}
    </div>
    <button class="k-btn ghost" onclick={close}>Cancel</button>
    <button class="k-btn primary" disabled={!canSave} onclick={save}>
      {#if saving}<span class="k-spinner"></span>{:else}<Icon name="release" />{/if}
      {saveLabel}
    </button>
  </header>

  <form
    class="content"
    onsubmit={(e) => {
      e.preventDefault();
      save();
    }}
  >
    {#if store.releasesStatus === "loading" && !host}
      <div class="empty-note"><span class="k-spinner" role="status" aria-label="Loading"></span></div>
    {:else if store.releasesStatus === "error"}
      <div class="k-banner danger">
        <Icon name="error" size={20} />
        <div class="body selectable">{store.releasesError}</div>
        <button type="button" class="k-btn" onclick={() => store.loadReleases()}>Try again</button>
      </div>
    {:else if host?.problem}
      <div class="k-banner warning">
        <Icon name="warning" size={20} />
        <div class="body">{host.problem}</div>
      </div>
    {/if}

    <div class="row">
      <label class="k-field">
        <span class="label">Tag</span>
        <input
          class="k-input mono"
          placeholder={versions?.patch ?? "v1.0.0"}
          spellcheck="false"
          bind:value={tag}
          disabled={!!existing}
          aria-invalid={taken ? "true" : undefined}
        />
      </label>
      {#if !existing && !tagRef && targets.length}
        <label class="k-field">
          <span class="label">Tag this</span>
          <select class="k-input" bind:value={target}>
            {#each targets as t (t.value)}<option value={t.value}>{t.label}</option>{/each}
          </select>
        </label>
      {/if}
    </div>

    {#if versions && !trimmedTag}
      <div class="chips">
        <span class="k-muted">Next version:</span>
        <button type="button" class="k-btn ghost mono" onclick={() => (tag = versions.patch)}>{versions.patch}</button>
        <button type="button" class="k-btn ghost mono" onclick={() => (tag = versions.minor)}>{versions.minor}</button>
        <button type="button" class="k-btn ghost mono" onclick={() => (tag = versions.major)}>{versions.major}</button>
      </div>
    {/if}

    {#if taken}
      <div class="hint error">There's already a release for {trimmedTag}. Pick another tag, or edit that release.</div>
    {:else if !existing && trimmedTag}
      <div class="hint">
        {#if tagRef}
          Uses the existing tag {trimmedTag} at {shortHash(tagRef.target)}, which is pushed to {host?.remote ?? "the remote"} if it isn't there yet.
        {:else}
          Creates the tag {trimmedTag} and pushes it to {host?.remote ?? "the remote"}.
        {/if}
      </div>
    {/if}

    <label class="k-field">
      <span class="label">Title</span>
      <input class="k-input" placeholder={trimmedTag || "Release title"} bind:value={name} />
    </label>

    <div class="k-field">
      <div class="notes-head">
        <span class="label">Release notes</span>
        <button type="button" class="k-btn ghost" disabled={generating || (!tagRef && !target)} onclick={generate}>
          {#if generating}<span class="k-spinner"></span>{:else}<Icon name="commit" />{/if}
          List commits since last tag
        </button>
      </div>
      <textarea class="k-input mono" rows="16" placeholder="What changed in this release? Markdown works." bind:value={body}></textarea>
    </div>

    {#if host?.supportsDrafts}
      <div class="checks">
        <label class="k-check">
          <input type="checkbox" bind:checked={draft} disabled={!!existing && !existing.draft} />
          <span>Draft (only visible to people who can edit the repository)</span>
        </label>
        <label class="k-check">
          <input type="checkbox" bind:checked={prerelease} />
          <span>Pre-release (not ready for production)</span>
        </label>
      </div>
    {/if}
  </form>
</div>

<style>
  .editor {
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
    font-weight: 600;
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
    gap: var(--space-12);
    max-width: 820px;
  }
  .row {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
    gap: var(--space-12);
  }
  .chips {
    display: flex;
    align-items: center;
    gap: var(--space-4);
    flex-wrap: wrap;
    margin-top: calc(-1 * var(--space-6));
    font-size: 12px;
  }
  .chips .k-btn {
    height: 24px;
    padding: 0 var(--space-8);
  }
  .hint {
    margin-top: calc(-1 * var(--space-6));
    font-size: 12px;
    line-height: 16px;
    color: var(--text-3);
  }
  .hint.error {
    color: var(--danger);
  }
  .notes-head {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-8);
  }
  .notes-head .label {
    font-size: 12px;
    line-height: 16px;
    font-weight: 500;
    color: var(--text-2);
  }
  textarea {
    min-height: 240px;
  }
  .checks {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
  }
  .k-check {
    color: var(--text-2);
  }
  .empty-note .k-spinner {
    display: inline-block;
  }
</style>
