<script lang="ts">
  import type { Commit } from "../api";
  import type { RepoStore } from "../repo.svelte";
  import { segmentPath } from "../graph";
  import { showMenu } from "../menu.svelte";
  import { toast } from "../state.svelte";
  import { fullDate, relativeTime, shortHash } from "../util";
  import Icon from "./Icon.svelte";

  let { store }: { store: RepoStore } = $props();

  const ROW_H = 30;
  const LANE_W = 14;
  const MAX_LANES_SHOWN = 12;
  const OVERSCAN = 10;

  let scroller: HTMLDivElement | undefined = $state();
  let scrollTop = $state(0);
  let viewH = $state(600);
  let search = $state("");

  let wip = $derived(store.dirty || !!store.status?.state);
  let offset = $derived(wip ? 1 : 0);
  let total = $derived(store.commits.length + offset);
  let lanesShown = $derived(Math.min(store.layout.maxLanes, MAX_LANES_SHOWN));
  let graphW = $derived(lanesShown * LANE_W + 6);
  let first = $derived(Math.max(0, Math.floor(scrollTop / ROW_H) - OVERSCAN));
  let last = $derived(Math.min(total, Math.ceil((scrollTop + viewH) / ROW_H) + OVERSCAN));
  let headIndex = $derived(store.headHash ? store.commits.findIndex((c) => c.hash === store.headHash) : -1);
  let wipCol = $derived(headIndex >= 0 ? store.layout.rows[headIndex]?.col ?? 0 : 0);
  let selectedHash = $derived(store.selected?.kind === "commit" ? store.selected.hash : null);

  interface Pill {
    label: string;
    kind: "local" | "remote" | "tag" | "head";
    remote: boolean;
    current: boolean;
  }

  /** Collapse "main" + "origin/main" into one pill, mark the checked-out branch. */
  function pills(c: Commit): Pill[] {
    if (!c.refs.length) return [];
    const out: Pill[] = [];
    const remotes = c.refs.filter((r) => r.startsWith("refs/remotes/")).map((r) => r.slice(13));
    const usedRemotes = new Set<string>();
    const isHead = c.refs.includes("HEAD");
    for (const r of c.refs) {
      if (!r.startsWith("refs/heads/")) continue;
      const name = r.slice(11);
      const tracking = remotes.find((rr) => rr.slice(rr.indexOf("/") + 1) === name);
      if (tracking) usedRemotes.add(tracking);
      out.push({ label: name, kind: "local", remote: !!tracking, current: isHead && store.currentBranch === name });
    }
    for (const r of remotes) {
      if (!usedRemotes.has(r) && !r.endsWith("/HEAD")) out.push({ label: r, kind: "remote", remote: true, current: false });
    }
    for (const r of c.refs) {
      if (r.startsWith("refs/tags/")) out.push({ label: r.slice(10), kind: "tag", remote: false, current: false });
    }
    if (isHead && !store.currentBranch) out.unshift({ label: "HEAD", kind: "head", remote: false, current: true });
    out.sort((a, b) => Number(b.current) - Number(a.current));
    return out;
  }

  function refFor(p: Pill) {
    if (p.kind === "local") return store.refs.find((r) => r.kind === "local" && r.short === p.label);
    if (p.kind === "remote") return store.refs.find((r) => r.kind === "remote" && r.short === p.label);
    if (p.kind === "tag") return store.refs.find((r) => r.kind === "tag" && r.short === p.label);
  }

  // Scroll a requested commit into view.
  $effect(() => {
    const target = store.scrollTo;
    if (!target || !scroller) return;
    store.scrollTo = null;
    const i = store.commits.findIndex((c) => c.hash === target);
    if (i === -1) {
      toast("That commit isn't in the loaded history yet. Scroll down to load more.");
      return;
    }
    const y = (i + offset) * ROW_H;
    if (y < scroller.scrollTop || y > scroller.scrollTop + viewH - ROW_H) {
      scroller.scrollTop = Math.max(0, y - viewH / 2);
    }
  });

  function onScroll() {
    if (!scroller) return;
    scrollTop = scroller.scrollTop;
    if (store.hasMore && scrollTop + viewH > total * ROW_H - ROW_H * 20) void store.loadMore();
  }

  function selectIndex(i: number) {
    if (i < 0 || i >= total) return;
    if (wip && i === 0) store.select({ kind: "wip" });
    else store.selectCommit(store.commits[i - offset].hash);
  }

  function onKey(e: KeyboardEvent) {
    if (e.key !== "ArrowDown" && e.key !== "ArrowUp") return;
    e.preventDefault();
    let cur = store.selected?.kind === "wip" ? 0 : store.commits.findIndex((c) => c.hash === selectedHash) + offset;
    selectIndex(cur + (e.key === "ArrowDown" ? 1 : -1));
  }

  function findNext(e: Event) {
    e.preventDefault();
    const q = search.trim().toLowerCase();
    if (!q) return;
    const start = selectedHash ? store.commits.findIndex((c) => c.hash === selectedHash) + 1 : 0;
    const n = store.commits.length;
    for (let k = 0; k < n; k++) {
      const c = store.commits[(start + k) % n];
      if (c.subject.toLowerCase().includes(q) || c.author.toLowerCase().includes(q) || c.hash.startsWith(q)) {
        store.selectCommit(c.hash);
        return;
      }
    }
    toast(`No loaded commits match "${search}"`);
  }
</script>

<div class="graph">
  <form class="search" onsubmit={findNext}>
    <Icon name="search" size={13} />
    <input placeholder="Find commit by message, author or hash — Enter for next" bind:value={search} spellcheck="false" />
  </form>

  <div
    class="scroller"
    bind:this={scroller}
    bind:clientHeight={viewH}
    onscroll={onScroll}
    onkeydown={onKey}
    tabindex="0"
    role="listbox"
    aria-label="Commit history"
  >
    <div class="spacer" style="height:{total * ROW_H}px">
      {#each { length: last - first } as _, k (first + k)}
        {@const i = first + k}
        {#if wip && i === 0}
          <div
            class="row wip"
            class:selected={store.selected?.kind === "wip"}
            style="top:0;height:{ROW_H}px"
            role="option"
            aria-selected={store.selected?.kind === "wip"}
            tabindex="-1"
            onclick={() => store.select({ kind: "wip" })}
            onkeydown={() => {}}
          >
            <svg width={graphW} height={ROW_H} class="lanes">
              {#if headIndex === 0}
                <path d="M{wipCol * LANE_W + LANE_W / 2} {ROW_H / 2}V{ROW_H}" class="dash" />
              {/if}
              <circle cx={wipCol * LANE_W + LANE_W / 2} cy={ROW_H / 2} r="5" class="wip-dot" />
            </svg>
            <span class="msg">
              <span class="wip-label">// WIP</span>
              <span class="muted">
                {store.status?.files.length ?? 0} changed file{store.status?.files.length === 1 ? "" : "s"}
              </span>
            </span>
          </div>
        {:else}
          {@const c = store.commits[i - offset]}
          {@const row = store.layout.rows[i - offset]}
          {#if c && row}
            <div
              class="row"
              class:selected={selectedHash === c.hash}
              class:head={c.hash === store.headHash}
              style="top:{i * ROW_H}px;height:{ROW_H}px"
              role="option"
              aria-selected={selectedHash === c.hash}
              tabindex="-1"
              onclick={() => store.select({ kind: "commit", hash: c.hash })}
              ondblclick={() => store.checkoutCommit(c.hash)}
              oncontextmenu={(e) => showMenu(e, store.commitMenu(c))}
              onkeydown={() => {}}
            >
              <svg width={graphW} height={ROW_H} class="lanes">
                {#each row.segments as s, si (si)}
                  {#if s.x1 < lanesShown && s.x2 < lanesShown}
                    <path d={segmentPath(s, LANE_W, ROW_H)} style="stroke:var(--lane-{s.color})" />
                  {/if}
                {/each}
                {#if row.col < lanesShown}
                  {@const cx = row.col * LANE_W + LANE_W / 2}
                  {#if c.hash === store.headHash}
                    <circle {cx} cy={ROW_H / 2} r="7" class="head-ring" style="stroke:var(--lane-{row.color})" />
                  {/if}
                  {#if c.parents.length > 1}
                    <circle {cx} cy={ROW_H / 2} r="4" class="merge" style="stroke:var(--lane-{row.color})" />
                  {:else}
                    <circle {cx} cy={ROW_H / 2} r="4.5" style="fill:var(--lane-{row.color})" />
                  {/if}
                {/if}
              </svg>
              <span class="msg">
                {#each pills(c) as p (p.kind + p.label)}
                  <button
                    class="ref-pill"
                    class:current={p.current}
                    style="--c:var(--lane-{row.color})"
                    title={p.label}
                    onclick={(e) => {
                      e.stopPropagation();
                      store.select({ kind: "commit", hash: c.hash });
                    }}
                    ondblclick={(e) => {
                      e.stopPropagation();
                      const r = refFor(p);
                      if (r) store.checkout(r);
                    }}
                    oncontextmenu={(e) => {
                      const r = refFor(p);
                      if (r) showMenu(e, store.refMenu(r));
                    }}
                  >
                    {#if p.current}<Icon name="check" size={11} stroke={2.6} />{/if}
                    {#if p.kind === "tag"}<Icon name="tag" size={11} />{/if}
                    <span class="ellipsis">{p.label}</span>
                    {#if p.remote}<Icon name="cloud" size={11} />{/if}
                  </button>
                {/each}
                <span class="subject ellipsis">{c.subject}</span>
              </span>
              <span class="author ellipsis" title={c.email}>{c.author}</span>
              <span class="date" title={fullDate(c.time)}>{relativeTime(c.time)}</span>
              <span class="hash mono">{shortHash(c.hash)}</span>
            </div>
          {/if}
        {/if}
      {/each}
    </div>
    {#if store.loaded && total === 0}
      <div class="empty">No commits yet. Make your first commit from the panel on the right.</div>
    {/if}
  </div>
</div>

<style>
  .graph {
    display: flex;
    flex-direction: column;
    min-height: 0;
    flex: 1;
  }
  .search {
    display: flex;
    align-items: center;
    gap: 7px;
    padding: 0 12px;
    color: var(--muted);
    border-bottom: 1px solid var(--border);
  }
  .search input {
    border: 0;
    background: none;
    padding: 7px 0;
    box-shadow: none;
  }
  .scroller {
    flex: 1;
    overflow-y: auto;
    overflow-x: hidden;
    outline: none;
    position: relative;
  }
  .spacer {
    position: relative;
  }
  .row {
    position: absolute;
    left: 0;
    right: 0;
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 0 12px 0 8px;
    white-space: nowrap;
  }
  .row:hover {
    background: var(--hover);
  }
  .row.selected {
    background: var(--selected);
  }
  .row.head .subject {
    font-weight: 600;
  }
  .lanes {
    flex-shrink: 0;
    overflow: visible;
  }
  .lanes path {
    fill: none;
    stroke-width: 2;
  }
  .lanes .dash {
    stroke: var(--muted);
    stroke-dasharray: 3 3;
  }
  .lanes .merge {
    fill: var(--bg);
    stroke-width: 2.2;
  }
  .lanes .head-ring {
    fill: none;
    stroke-width: 1.6;
    opacity: 0.6;
  }
  .wip-dot {
    fill: var(--bg);
    stroke: var(--muted);
    stroke-width: 2;
    stroke-dasharray: 2.5 2;
  }
  .msg {
    flex: 1;
    min-width: 0;
    display: flex;
    align-items: center;
    gap: 5px;
  }
  .wip-label {
    font-weight: 600;
    color: var(--accent);
    font-family: "JetBrains Mono", ui-monospace, monospace;
    font-size: 12px;
  }
  .ref-pill {
    color: var(--c);
    border-color: color-mix(in srgb, var(--c) 55%, transparent);
    background: color-mix(in srgb, var(--c) 14%, transparent);
    cursor: pointer;
    font-family: inherit;
  }
  .ref-pill.current {
    background: var(--c);
    color: #fff;
    border-color: var(--c);
  }
  .author {
    width: 130px;
    color: var(--muted);
    flex-shrink: 0;
  }
  .date {
    width: 100px;
    color: var(--muted);
    flex-shrink: 0;
    font-size: 12px;
  }
  .hash {
    width: 58px;
    color: var(--faint);
    flex-shrink: 0;
  }
  @media (max-width: 1250px) {
    .hash,
    .author {
      display: none;
    }
  }
</style>
