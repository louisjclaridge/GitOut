<script lang="ts">
  import { onMount, tick } from "svelte";
  import { openPath } from "@tauri-apps/plugin-opener";
  import { api, type ConflictFile, type ConflictSide } from "../api";
  import type { RepoStore } from "../repo.svelte";
  import { showMenu, type MenuItem } from "../menu.svelte";
  import { confirm, errorText, run, runOk, toast } from "../state.svelte";
  import { assemble, bare, parseConflicts, resolvedText, splitLines, type Common, type Parsed, type Pick, type Resolution } from "../conflict";
  import { basename } from "../util";
  import Icon from "./Icon.svelte";

  let { store, file }: { store: RepoStore; file: string } = $props();

  /** Unchanged lines kept visible around each conflict when the rest is folded away. */
  const CONTEXT = 3;

  let info = $state<ConflictFile | null>(null);
  let error = $state("");
  let parsed = $state.raw<Parsed | null>(null);
  let resolutions = $state<(Resolution | null)[]>([]);
  /** The working copy as last loaded or saved, to tell whether there are unsaved edits. */
  let saved = $state("");
  let textMode = $state(false);
  let rawText = $state("");
  let expanded = $state<Record<number, boolean>>({});
  let busy = $state(false);
  let panes: HTMLElement | undefined = $state();

  async function load() {
    try {
      const f = await api.conflictFile(store.path, file);
      const p = f.working !== null ? parseConflicts(f.working) : null;
      info = f;
      error = "";
      saved = f.working ?? "";
      rawText = saved;
      parsed = p;
      resolutions = p ? Array.from({ length: p.conflicts }, () => null) : [];
      // Unbalanced markers can't be split into blocks, so edit the text directly.
      textMode = f.working !== null && !f.binary && !p;
      expanded = {};
      await tick();
      goTo(0);
    } catch (e) {
      error = errorText(e);
    }
  }

  onMount(() => {
    void load();
  });

  let mode = $derived(
    !info
      ? "loading"
      : !info.ours.exists && !info.theirs.exists
        ? "gone"
        : textMode
          ? "text"
          : parsed && parsed.conflicts > 0
            ? "blocks"
            : "whole",
  );

  let total = $derived(parsed?.conflicts ?? 0);
  let resolved = $derived(resolutions.filter(Boolean).length);
  let current = $derived(textMode ? rawText : parsed ? assemble(parsed, resolutions) : saved);
  let dirty = $derived(!!info && current !== saved);
  let eol = $derived(parsed?.eol ?? "\n");
  let ours = $derived(info?.ours);
  let theirs = $derived(info?.theirs);

  /** Conflict number for each segment (-1 for common text). */
  let conflictAt = $derived.by(() => {
    let n = 0;
    return parsed?.segments.map((s) => (s.kind === "conflict" ? n++ : -1)) ?? [];
  });

  function folded(seg: Common, i: number, count: number) {
    const keepHead = i === 0 ? 0 : CONTEXT;
    const keepTail = i === count - 1 ? 0 : CONTEXT;
    const n = seg.lines.length;
    if (expanded[i] || n <= keepHead + keepTail + 2) return { head: seg.lines, hidden: 0, tail: [] as string[] };
    return { head: seg.lines.slice(0, keepHead), hidden: n - keepHead - keepTail, tail: seg.lines.slice(n - keepTail) };
  }

  function label(side: ConflictSide | undefined) {
    return side ? side.title : "";
  }

  function pickLabel(p: Pick): string {
    switch (p) {
      case "ours":
        return label(ours);
      case "theirs":
        return label(theirs);
      case "ours-theirs":
        return `Both, ${label(ours).toLowerCase()} first`;
      case "theirs-ours":
        return `Both, ${label(theirs).toLowerCase()} first`;
      case "base":
        return "Common ancestor";
      case "none":
        return "Neither";
      case "custom":
        return "Edited";
    }
  }

  function pick(i: number, p: Pick) {
    resolutions[i] = { pick: p, text: "" };
  }

  function edit(i: number, text: string) {
    resolutions[i] = { pick: "custom", text };
  }

  function pickAll(p: "ours" | "theirs") {
    resolutions = resolutions.map((r) => r ?? { pick: p, text: "" });
  }

  function moreMenu(i: number): MenuItem[] {
    const seg = parsed?.segments.find((_, si) => conflictAt[si] === i);
    const base = seg?.kind === "conflict" && seg.base !== null;
    return [
      { label: `Both, ${label(theirs).toLowerCase()} first`, icon: "merge", action: () => pick(i, "theirs-ours") },
      { label: "Common ancestor's version", icon: "commit", action: () => pick(i, "base"), disabled: !base },
      { label: "Neither (remove both)", icon: "minus", action: () => pick(i, "none") },
    ];
  }

  // ------------------------------------------------------------ navigation

  let at = $state(0);

  function goTo(i: number) {
    if (!total) return;
    at = ((i % total) + total) % total;
    panes?.querySelector(`[data-conflict="${at}"]`)?.scrollIntoView({ block: "center", behavior: "smooth" });
  }

  function step(dir: 1 | -1) {
    // Prefer the next block that still needs a decision.
    for (let k = 1; k <= total; k++) {
      const i = (((at + dir * k) % total) + total) % total;
      if (!resolutions[i]) return goTo(i);
    }
    goTo(at + dir);
  }

  // ------------------------------------------------------------ text mode

  function toggleText() {
    if (!textMode) {
      rawText = current;
      textMode = true;
      return;
    }
    const p = parseConflicts(rawText);
    if (!p) {
      toast("The conflict markers in the text don't pair up, so it can't be shown side by side. Fix them or keep editing as text.", "error");
      return;
    }
    parsed = p;
    resolutions = Array.from({ length: p.conflicts }, () => null);
    expanded = {};
    textMode = false;
  }

  // ------------------------------------------------------------ saving

  /** After resolving, move on to the next conflicted file, or back to the history. */
  function next() {
    const other = store.conflicted.find((f) => f.path !== file);
    store.center = other ? { kind: "merge", file: other.path } : { kind: "graph" };
  }

  async function save() {
    if (busy || !dirty) return;
    busy = true;
    const text = current;
    if (await runOk(() => api.conflictSave(store.path, file, text, false), { done: `Saved ${basename(file)}` })) saved = text;
    busy = false;
  }

  async function markResolved() {
    if (busy) return;
    const text = current;
    if (mode === "blocks" && resolved < total) return;
    if (mode === "text") {
      const p = parseConflicts(text);
      if (
        (!p || p.conflicts > 0) &&
        !(await confirm("Conflict markers are still there", "The text still contains <<<<<<< / >>>>>>> markers. Mark the file resolved anyway?", "Mark resolved", true))
      )
        return;
    }
    busy = true;
    const ok = await runOk(() => api.conflictSave(store.path, file, text, true), { done: `Resolved ${basename(file)}` });
    busy = false;
    if (ok) next();
  }

  async function take(side: "ours" | "theirs") {
    const s = side === "ours" ? info?.ours : info?.theirs;
    if (!s || busy) return;
    if (dirty && !(await confirm("Discard your edits?", `Taking the ${s.title.toLowerCase()} version replaces the whole file, including your edits here.`, "Replace", true))) return;
    busy = true;
    const ok = await runOk(() => api.conflictTake(store.path, file, side), {
      done: s.exists ? `Kept the ${s.title.toLowerCase()} version of ${basename(file)}` : `Deleted ${basename(file)}`,
    });
    busy = false;
    if (ok) next();
  }

  async function restart() {
    if (!(await confirm("Start over?", "This throws away every edit to this file and writes its conflict markers again, with the common ancestor's lines included.", "Start over", true))) return;
    if (await runOk(() => api.conflictRestart(store.path, file))) await load();
  }

  async function close() {
    if (dirty && !(await confirm("Discard your edits?", `Your changes to ${basename(file)} haven't been saved.`, "Discard", true))) return;
    store.center = { kind: "graph" };
  }

  function fileMenu(): MenuItem[] {
    const blocks = mode === "blocks";
    return [
      { label: `Use ${label(ours).toLowerCase()} for every block`, icon: "back", action: () => pickAll("ours"), disabled: !blocks },
      { label: `Use ${label(theirs).toLowerCase()} for every block`, icon: "forward", action: () => pickAll("theirs"), disabled: !blocks },
      "sep",
      { label: textMode ? "Edit side by side" : "Edit as text", icon: "file", action: toggleText, disabled: mode !== "blocks" && mode !== "text" },
      { label: "Open in default app", icon: "external", action: () => run(() => openPath(`${store.path.replace(/[\\/]$/, "")}/${file}`)) },
      "sep",
      { label: "Start over…", icon: "undo", danger: true, action: restart, disabled: !info?.ours.exists || !info?.theirs.exists },
    ];
  }

  function onKey(e: KeyboardEvent) {
    if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
      e.preventDefault();
      void save();
    }
  }

  /** Grow a textarea to fit its text, refitting when the text or width changes. */
  function autosize(_text: string) {
    return (el: HTMLTextAreaElement) => {
      const fit = () => {
        el.style.height = "auto";
        el.style.height = `${el.scrollHeight}px`;
      };
      fit();
      let width = el.clientWidth;
      const ro = new ResizeObserver(() => {
        if (el.clientWidth === width) return;
        width = el.clientWidth;
        fit();
      });
      ro.observe(el);
      return () => ro.disconnect();
    };
  }

  function lineCount(text: string | null) {
    return text === null ? 0 : splitLines(text).length;
  }
</script>

<svelte:window onkeydown={onKey} />

{#snippet code(lines: string[])}
  {#if lines.length}<pre class="code selectable">{lines.map(bare).join("\n")}</pre>{:else}<div class="nothing">No lines</div>{/if}
{/snippet}

<div class="merge">
  <header>
    <button class="k-icon-btn" aria-label="Close the merge editor" title="Close the merge editor" onclick={close}><Icon name="back" /></button>
    <div class="title">
      <strong class="ellipsis mono selectable" title={file}>{file}</strong>
      <span class="ellipsis">
        {#if mode === "blocks"}
          {resolved} of {total} conflict{total === 1 ? "" : "s"} resolved
        {:else if mode === "text"}
          Editing as text
        {:else}
          Merge conflict
        {/if}
        {#if dirty}· unsaved{/if}
      </span>
    </div>
    {#if mode === "blocks" && total > 1}
      <span class="nav">
        <button class="k-icon-btn" aria-label="Previous conflict" title="Previous conflict" onclick={() => step(-1)}><Icon name="up" /></button>
        <button class="k-icon-btn" aria-label="Next conflict" title="Next conflict" onclick={() => step(1)}><Icon name="down" /></button>
      </span>
    {/if}
    {#if info}
      <button class="k-icon-btn" aria-label="More actions" title="More actions" onclick={(e) => showMenu(e, fileMenu())}><Icon name="more" /></button>
    {/if}
    {#if mode === "blocks" || mode === "text"}
      <button class="k-btn ghost" disabled={busy || !dirty} title="Save without marking resolved (Ctrl+S)" onclick={save}>Save</button>
      <button
        class="k-btn primary"
        disabled={busy || (mode === "blocks" && resolved < total)}
        title={mode === "blocks" && resolved < total ? "Resolve every conflict first" : "Save and stage the file"}
        onclick={markResolved}
      >
        <Icon name="check" /> Mark resolved
      </button>
    {/if}
  </header>

  {#if error}
    <div class="empty-note selectable">{error}</div>
  {:else if mode === "loading"}
    <div class="empty-note"><span class="k-spinner" role="status" aria-label="Loading conflict"></span></div>
  {:else if mode === "gone"}
    <div class="k-empty">
      <div class="art"><Icon name="success" size={24} /></div>
      <h3>Not conflicted</h3>
      <p>{file} has no conflicts left to resolve.</p>
      <div class="actions"><button class="k-btn" onclick={next}>Continue</button></div>
    </div>
  {:else if mode === "text"}
    <div class="text">
      <textarea class="k-input mono" spellcheck="false" aria-label="File contents" bind:value={rawText}></textarea>
    </div>
  {:else if mode === "whole" && info && ours && theirs}
    <div class="whole">
      <p class="why">
        {#if !ours.exists}
          {ours.title} deleted this file, and {theirs.title.toLowerCase()} changed it.
        {:else if !theirs.exists}
          {theirs.title} deleted this file, and {ours.title.toLowerCase()} changed it.
        {:else if info.binary || ours.text === null || theirs.text === null}
          Both sides changed this file, and it isn't text, so it can't be merged line by line.
        {:else}
          This file has no conflict markers left. Keep it as it is, or pick one side's version.
        {/if}
        Choose the version to keep.
      </p>
      <div class="cards">
        {#each [{ key: "ours" as const, s: ours }, { key: "theirs" as const, s: theirs }] as { key, s } (key)}
          <div class="card">
            <div class="card-head">
              <span class="k-label">{s.title}</span>
              {#if s.detail}<span class="detail ellipsis" title={s.detail}>{s.detail}</span>{/if}
            </div>
            {#if !s.exists}
              <div class="nothing">Deleted the file</div>
            {:else if s.text === null}
              <div class="nothing">Binary file</div>
            {:else}
              <pre class="code preview selectable">{splitLines(s.text).slice(0, 40).map(bare).join("\n")}</pre>
              {#if lineCount(s.text) > 40}<div class="nothing">…and {lineCount(s.text) - 40} more lines</div>{/if}
            {/if}
            <button class="k-btn tinted" disabled={busy} onclick={() => take(key)}>
              {s.exists ? `Keep the ${s.title.toLowerCase()} version` : "Delete the file"}
            </button>
          </div>
        {/each}
      </div>
      {#if info.working !== null && !info.binary}
        <div class="alt">
          <button class="k-btn" disabled={busy} onclick={markResolved}><Icon name="check" /> Mark resolved as it is</button>
          <button class="k-btn ghost" onclick={() => (textMode = true)}><Icon name="pencil" /> Edit it</button>
        </div>
      {/if}
    </div>
  {:else if parsed && ours && theirs}
    <div class="panes" bind:this={panes}>
      <div class="colhead">
        <span class="k-label">{ours.title}</span>
        <span class="detail ellipsis" title={ours.detail}>{ours.detail}</span>
      </div>
      <div class="colhead mid"><span class="k-label">Result</span></div>
      <div class="colhead">
        <span class="k-label">{theirs.title}</span>
        <span class="detail ellipsis" title={theirs.detail}>{theirs.detail}</span>
      </div>

      {#each parsed.segments as seg, si (si)}
        {#if seg.kind === "common"}
          {@const v = folded(seg, si, parsed.segments.length)}
          {#if v.head.length}
            <div class="cell">{@render code(v.head)}</div>
            <div class="cell mid">{@render code(v.head)}</div>
            <div class="cell">{@render code(v.head)}</div>
          {/if}
          {#if v.hidden}
            <button class="fold" onclick={() => (expanded[si] = true)}>
              <Icon name="more" size={14} /> {v.hidden} unchanged line{v.hidden === 1 ? "" : "s"}
            </button>
          {/if}
          {#if v.tail.length}
            <div class="cell">{@render code(v.tail)}</div>
            <div class="cell mid">{@render code(v.tail)}</div>
            <div class="cell">{@render code(v.tail)}</div>
          {/if}
        {:else}
          {@const ci = conflictAt[si]}
          {@const res = resolutions[ci]}
          {@const usesOurs = !!res && ["ours", "ours-theirs", "theirs-ours"].includes(res.pick)}
          {@const usesTheirs = !!res && ["theirs", "ours-theirs", "theirs-ours"].includes(res.pick)}
          <div class="cell side ours" class:used={usesOurs} class:dropped={!!res && !usesOurs} data-conflict={ci}>
            <div class="bar">
              <span class="n">{seg.ours.length} line{seg.ours.length === 1 ? "" : "s"}</span>
              <button class="k-btn tinted" title="Use {ours.title.toLowerCase()}'s lines" onclick={() => pick(ci, "ours")}>
                Use <Icon name="forward" />
              </button>
            </div>
            {@render code(seg.ours)}
          </div>

          <div class="cell mid block" class:open={!res} class:current={at === ci}>
            {#if res}
              <div class="bar">
                <span class="n">{pickLabel(res.pick)}</span>
                <button class="k-btn ghost" title="Undo this choice" onclick={() => (resolutions[ci] = null)}><Icon name="undo" /> Undo</button>
              </div>
              <textarea
                class="edit mono"
                spellcheck="false"
                aria-label="Result of conflict {ci + 1}"
                rows="1"
                placeholder="(no lines)"
                bind:value={() => resolvedText(seg, res, eol), (v) => edit(ci, v)}
                {@attach autosize(resolvedText(seg, res, eol))}
              ></textarea>
            {:else}
              <div class="choose">
                <span class="k-label">Conflict {ci + 1} of {total}</span>
                <div class="choices">
                  <button class="k-btn" onclick={() => pick(ci, "ours")}><Icon name="back" /> {ours.title}</button>
                  <button class="k-btn" onclick={() => pick(ci, "ours-theirs")}>Both</button>
                  <button class="k-btn" onclick={() => pick(ci, "theirs")}>{theirs.title} <Icon name="forward" /></button>
                  <button class="k-icon-btn" aria-label="Other choices" title="Other choices" onclick={(e) => showMenu(e, moreMenu(ci))}>
                    <Icon name="more" />
                  </button>
                </div>
                <button class="k-link" onclick={() => edit(ci, [...seg.ours, ...seg.theirs].map(bare).join("\n"))}>Write it by hand</button>
              </div>
            {/if}
          </div>

          <div class="cell side theirs" class:used={usesTheirs} class:dropped={!!res && !usesTheirs}>
            <div class="bar">
              <button class="k-btn tinted" title="Use {theirs.title.toLowerCase()}'s lines" onclick={() => pick(ci, "theirs")}>
                <Icon name="back" /> Use
              </button>
              <span class="n">{seg.theirs.length} line{seg.theirs.length === 1 ? "" : "s"}</span>
            </div>
            {@render code(seg.theirs)}
          </div>
        {/if}
      {/each}
    </div>
  {/if}
</div>

<style>
  .merge {
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
  .nav {
    display: flex;
  }

  /* Three columns; each segment is one grid row, so the panes stay aligned
     however long either side's block is. */
  .panes {
    flex: 1;
    overflow: auto;
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    align-content: start;
    padding-bottom: var(--space-24);
  }
  .colhead {
    position: sticky;
    top: 0;
    z-index: 2;
    display: flex;
    align-items: baseline;
    gap: var(--space-8);
    min-width: 0;
    padding: var(--space-8) var(--space-12);
    background: var(--surface-2);
    border-bottom: 1px solid var(--border);
  }
  .detail {
    min-width: 0;
    font-size: 12px;
    color: var(--text-3);
  }
  .cell {
    min-width: 0;
    padding: 0 var(--space-12);
  }
  .mid {
    border-left: 1px solid var(--border);
    border-right: 1px solid var(--border);
  }
  .code {
    margin: 0;
    font-family: var(--font-mono);
    font-size: 12px;
    line-height: 20px;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    tab-size: 4;
  }
  .cell:not(.side):not(.block) .code {
    color: var(--text-2);
  }
  .nothing {
    font-size: 12px;
    line-height: 20px;
    font-style: italic;
    color: var(--text-3);
  }
  .fold {
    grid-column: 1 / -1;
    display: flex;
    align-items: center;
    justify-content: center;
    gap: var(--space-6);
    height: 28px;
    margin: var(--space-4) 0;
    border: 0;
    background: var(--surface-2);
    color: var(--text-3);
    font-size: 12px;
  }
  .fold:hover {
    background: var(--surface-hover);
    color: var(--text);
  }

  /* Conflict blocks */
  .side,
  .block {
    margin: var(--space-6) 0;
    padding-top: var(--space-4);
    padding-bottom: var(--space-8);
    scroll-margin-top: 48px;
  }
  .side.ours {
    background: var(--accent-soft);
  }
  .side.theirs {
    background: var(--success-soft);
  }
  .side.dropped {
    background: var(--surface-2);
    opacity: 0.55;
  }
  .side.used {
    box-shadow: inset 0 0 0 1px var(--focus);
  }
  .bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: var(--space-8);
    min-height: 32px;
    margin-bottom: var(--space-4);
    font-family: var(--font-sans);
    font-size: 12px;
  }
  .bar .n {
    color: var(--text-3);
  }
  .bar .k-btn {
    height: 28px;
    padding: 0 var(--space-8);
  }
  .block.open {
    background: var(--warning-soft);
  }
  .block.current.open {
    box-shadow: inset 0 0 0 2px var(--warning);
  }
  .choose {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-8);
    padding: var(--space-8) 0;
    text-align: center;
  }
  .choices {
    display: flex;
    flex-wrap: wrap;
    justify-content: center;
    gap: var(--space-4);
  }
  .choices .k-btn {
    height: 28px;
    padding: 0 var(--space-8);
  }
  .edit {
    display: block;
    width: 100%;
    min-height: 20px;
    margin: 0;
    padding: var(--space-4) var(--space-6);
    border: 0;
    border-radius: var(--radius-xs);
    outline: 2px solid transparent;
    background: var(--surface-field);
    color: var(--text);
    font-size: 12px;
    line-height: 20px;
    resize: none;
    overflow: hidden;
    white-space: pre-wrap;
    overflow-wrap: anywhere;
    tab-size: 4;
  }
  .edit:focus {
    outline-color: var(--focus);
  }

  /* Text mode */
  .text {
    flex: 1;
    min-height: 0;
    display: flex;
    padding: var(--space-12);
  }
  .text textarea {
    flex: 1;
    min-height: 0;
    resize: none;
    line-height: 20px;
    tab-size: 4;
  }

  /* Whole-file choice */
  .whole {
    flex: 1;
    overflow: auto;
    padding: var(--space-16) var(--space-24) var(--space-32);
    display: flex;
    flex-direction: column;
    gap: var(--space-16);
  }
  .why {
    max-width: 640px;
    color: var(--text-2);
  }
  .cards {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
    gap: var(--space-12);
  }
  .card {
    display: flex;
    flex-direction: column;
    gap: var(--space-8);
    min-width: 0;
    padding: var(--space-12);
    border-radius: var(--radius-md);
    background: var(--surface-2);
  }
  .card-head {
    display: flex;
    align-items: baseline;
    gap: var(--space-8);
    min-width: 0;
  }
  .preview {
    max-height: 320px;
    overflow: auto;
    padding: var(--space-8);
    border-radius: var(--radius-sm);
    background: var(--surface);
  }
  .card .k-btn {
    align-self: flex-start;
  }
  .alt {
    display: flex;
    gap: var(--space-8);
    flex-wrap: wrap;
  }
  .empty-note .k-spinner {
    display: inline-block;
  }
</style>
