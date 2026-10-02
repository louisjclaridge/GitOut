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
  <button class="head" onclick={() => toggle(key)}>
    <span class="chev" class:open={open[key]}><Icon name="chevron" size={12} /></span>
    <span class="section-title">{label}</span>
    <span class="n">{count}</span>
  </button>
{/snippet}

{#snippet refRow(r: Ref, label: string, indent = false)}
  <button
    class="item"
    class:current={r.current}
    class:indent
    title={r.short}
    onclick={() => store.selectCommit(r.target)}
    ondblclick={() => store.checkout(r)}
    oncontextmenu={(e) => showMenu(e, store.refMenu(r))}
  >
    {#if r.current}<Icon name="check" size={13} />{:else}<span class="dot"></span>{/if}
    <span class="ellipsis label">{label}</span>
    {#if r.ahead}<span class="ab" title="{r.ahead} to push">↑{r.ahead}</span>{/if}
    {#if r.behind}<span class="ab" title="{r.behind} to pull">↓{r.behind}</span>{/if}
  </button>
{/snippet}

<nav class="sidebar">
  <div class="filter">
    <Icon name="search" size={13} />
    <input placeholder="Filter branches" bind:value={filter} spellcheck="false" />
  </div>

  <div class="scroll">
    {@render header("local", "Local", local.length)}
    {#if open.local}
      {#each local as r (r.name)}{@render refRow(r, r.short)}{/each}
      {#if !local.length}<div class="none">No branches yet</div>{/if}
    {/if}

    {@render header("remote", "Remote", remotes.reduce((n, [, l]) => n + l.length, 0))}
    {#if open.remote}
      {#each remotes as [name, list] (name)}
        <div class="remote-name"><Icon name="cloud" size={13} /> {name}</div>
        {#each list as r (r.name)}{@render refRow(r, r.short.slice(name.length + 1), true)}{/each}
      {/each}
      {#if !remotes.length}<div class="none">No remotes</div>{/if}
    {/if}

    {@render header("tags", "Tags", tags.length)}
    {#if open.tags}
      {#each tags as r (r.name)}{@render refRow(r, r.short)}{/each}
    {/if}

    {@render header("stashes", "Stashes", store.stashes.length)}
    {#if open.stashes}
      {#each store.stashes as s (s.hash)}
        <button
          class="item"
          title={s.message}
          onclick={() => store.select({ kind: "commit", hash: s.hash })}
          oncontextmenu={(e) => showMenu(e, store.stashMenu(s))}
        >
          <Icon name="stash" size={13} />
          <span class="ellipsis label">{s.message.replace(/^On [^:]+: /, "").replace(/^WIP on [^:]+: /, "")}</span>
        </button>
      {/each}
    {/if}
  </div>
</nav>

<style>
  .sidebar {
    display: flex;
    flex-direction: column;
    min-height: 0;
    background: var(--panel);
    border-right: 1px solid var(--border);
  }
  .filter {
    display: flex;
    align-items: center;
    gap: 6px;
    margin: 10px;
    padding: 0 8px;
    border: 1px solid var(--border);
    border-radius: 6px;
    background: var(--bg);
    color: var(--muted);
  }
  .filter input {
    border: 0;
    padding: 5px 0;
    background: none;
    box-shadow: none;
  }
  .scroll {
    flex: 1;
    overflow: auto;
    padding: 0 6px 12px;
  }
  .head {
    display: flex;
    align-items: center;
    gap: 6px;
    width: 100%;
    padding: 8px 6px 4px;
    border: 0;
    background: none;
  }
  .chev {
    display: inline-flex;
    color: var(--muted);
    transition: transform 0.1s;
  }
  .chev.open {
    transform: rotate(90deg);
  }
  .n {
    margin-left: auto;
    font-size: 11px;
    color: var(--faint);
  }
  .item {
    display: flex;
    align-items: center;
    gap: 7px;
    width: 100%;
    padding: 4px 8px 4px 22px;
    border: 0;
    border-radius: 5px;
    background: none;
    text-align: left;
    min-width: 0;
  }
  .item.indent {
    padding-left: 34px;
  }
  .item:hover {
    background: var(--hover);
  }
  .item.current {
    color: var(--accent);
    font-weight: 600;
  }
  .label {
    flex: 1;
  }
  .dot {
    width: 6px;
    height: 6px;
    margin: 0 3.5px;
    border-radius: 50%;
    background: var(--faint);
    flex-shrink: 0;
  }
  .ab {
    font-size: 10.5px;
    color: var(--muted);
    font-variant-numeric: tabular-nums;
  }
  .remote-name {
    display: flex;
    align-items: center;
    gap: 6px;
    padding: 4px 8px 2px 22px;
    color: var(--muted);
    font-size: 12px;
  }
  .none {
    padding: 2px 8px 4px 24px;
    color: var(--faint);
    font-size: 12px;
  }
</style>
