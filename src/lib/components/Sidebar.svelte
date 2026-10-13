<script lang="ts">
  import type { Ref } from "../api";
  import type { RepoStore } from "../repo.svelte";
  import { showMenu } from "../menu.svelte";
  import Icon from "./Icon.svelte";

  let { store }: { store: RepoStore } = $props();

  let open = $state<Record<string, boolean>>({ local: true, remote: true, tags: false, stashes: true });
  let filter = $state("");

  const match = (r: Ref) => !filter || r.short.toLowerCase().includes(filter.toLowerCase());
  const byName = (a: Ref, b: Ref) => a.short.localeCompare(b.short);

  let local = $derived(store.refs.filter((r) => r.kind === "local" && match(r)).sort(byName));
  let tags = $derived(store.refs.filter((r) => r.kind === "tag" && match(r)));
  let remotes = $derived.by(() => {
    const groups = new Map<string, Ref[]>();
    for (const r of store.refs) {
      if (r.kind !== "remote" || !match(r)) continue;
      const [remote] = r.short.split("/");
      if (!groups.has(remote)) groups.set(remote, []);
      groups.get(remote)!.push(r);
    }
    for (const list of groups.values()) list.sort(byName);
    return [...groups.entries()];
  });

  function toggle(k: string) {
    open[k] = !open[k];
  }
</script>

{#snippet header(key: string, label: string, count: number)}
  <button class="head" aria-expanded={open[key]} onclick={() => toggle(key)}>
    <span class="chev" class:open={open[key]}><Icon name="chevron" size={12} /></span>
    <span class="k-label">{label}</span>
    <span class="n">{count}</span>
  </button>
{/snippet}

{#snippet refRow(r: Ref, label: string, indent = false)}
  <button
    class="k-item"
    class:indent
    aria-current={r.current ? "true" : undefined}
    title={r.short}
    onclick={() => store.selectCommit(r.target)}
    ondblclick={() => store.checkout(r)}
    oncontextmenu={(e) => showMenu(e, store.refMenu(r))}
  >
    <Icon name={r.current ? "check" : r.kind === "tag" ? "tag" : "branch"} size={14} />
    <span class="name">{label}</span>
    {#if r.ahead || r.behind}
      <span class="meta">
        {#if r.ahead}<span title="{r.ahead} to push">↑{r.ahead}</span>{/if}
        {#if r.behind}<span title="{r.behind} to pull">↓{r.behind}</span>{/if}
      </span>
    {/if}
  </button>
{/snippet}

<nav class="sidebar">
  <div class="filter k-search">
    <Icon name="search" size={14} />
    <input class="k-input" placeholder="Filter branches" aria-label="Filter branches" bind:value={filter} spellcheck="false" />
  </div>

  <div class="scroll">
    {@render header("local", "Local", local.length)}
    {#if open.local}
      <div class="k-list">
        {#each local as r (r.name)}{@render refRow(r, r.short)}{/each}
        {#if !local.length}<div class="none">No branches yet</div>{/if}
      </div>
    {/if}

    {@render header("remote", "Remote", remotes.reduce((n, [, l]) => n + l.length, 0))}
    {#if open.remote}
      <div class="k-list">
        {#each remotes as [name, list] (name)}
          <div class="remote-name"><Icon name="cloud" size={14} /> {name}</div>
          {#each list as r (r.name)}{@render refRow(r, r.short.slice(name.length + 1), true)}{/each}
        {/each}
        {#if !remotes.length}<div class="none">No remotes</div>{/if}
      </div>
    {/if}

    {@render header("tags", "Tags", tags.length)}
    {#if open.tags}
      <div class="k-list">
        {#each tags as r (r.name)}{@render refRow(r, r.short)}{/each}
        {#if !tags.length}<div class="none">No tags</div>{/if}
      </div>
    {/if}

    {@render header("stashes", "Stashes", store.stashes.length)}
    {#if open.stashes}
      <div class="k-list">
        {#each store.stashes as s (s.hash)}
          <button
            class="k-item"
            title={s.message}
            onclick={() => store.select({ kind: "commit", hash: s.hash })}
            oncontextmenu={(e) => showMenu(e, store.stashMenu(s))}
          >
            <Icon name="stash" size={14} />
            <span class="name">{s.message.replace(/^On [^:]+: /, "").replace(/^WIP on [^:]+: /, "")}</span>
          </button>
        {/each}
        {#if !store.stashes.length}<div class="none">No stashes</div>{/if}
      </div>
    {/if}
  </div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--bg);
    border-right: 1px solid var(--border);
  }
  .filter {
    margin: var(--space-8);
  }
  .scroll {
    flex: 1;
    overflow: auto;
    padding-bottom: var(--space-12);
  }
  .head {
    display: flex;
    align-items: center;
    gap: var(--space-6);
    width: 100%;
    padding: var(--space-12) var(--space-16) var(--space-6) var(--space-12);
    border: 0;
    background: none;
  }
  .chev {
    display: inline-flex;
    color: var(--text-3);
    transition: transform 120ms ease;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .n {
    margin-left: auto;
    font-size: 11px;
    color: var(--text-3);
    font-variant-numeric: tabular-nums;
  }
  .k-item.indent {
    padding-left: var(--space-24);
  }
  .meta {
    display: flex;
    gap: var(--space-4);
    font-variant-numeric: tabular-nums;
  }
  .remote-name {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    height: 28px;
    padding: 0 var(--space-8);
    color: var(--text-2);
    font-size: 12px;
  }
  .none {
    padding: var(--space-2) var(--space-8) var(--space-4);
    color: var(--text-3);
    font-size: 12px;
  }
</style>
