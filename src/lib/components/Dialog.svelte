<script lang="ts">
  import { app } from "../state.svelte";

  let texts: Record<string, string> = $state({});
  let checks: Record<string, boolean> = $state({});
  let form: HTMLFormElement | undefined = $state();

  $effect(() => {
    const d = app.dialog;
    if (!d) return;
    const fields = d.fields ?? [];
    texts = Object.fromEntries(fields.filter((f) => f.type !== "checkbox").map((f) => [f.key, f.value ?? ""]));
    checks = Object.fromEntries(fields.filter((f) => f.type === "checkbox").map((f) => [f.key, !!f.checked]));
    queueMicrotask(() => {
      const first = form?.querySelector<HTMLElement>("input:not([type=checkbox]), textarea");
      (first ?? form?.querySelector<HTMLElement>("button.confirm"))?.focus();
      if (first instanceof HTMLInputElement) first.select();
    });
  });

  function close(result: Record<string, string | boolean> | null) {
    const d = app.dialog;
    app.dialog = null;
    d?.resolve(result);
  }
</script>

<svelte:window onkeydown={(e) => app.dialog && e.key === "Escape" && close(null)} />

{#if app.dialog}
  {@const d = app.dialog}
  <div class="k-scrim" role="presentation" onmousedown={(e) => e.target === e.currentTarget && close(null)}>
    <div class="k-dialog" role="dialog" aria-modal="true" aria-labelledby="dialog-title">
      <form
        bind:this={form}
        onsubmit={(e) => {
          e.preventDefault();
          close({ ...texts, ...checks });
        }}
      >
        <h2 id="dialog-title">{d.title}</h2>
        {#if d.message}<p class="msg selectable">{d.message}</p>{/if}
        {#each d.fields ?? [] as f (f.key)}
          {#if f.type === "checkbox"}
            <label class="k-check"><input type="checkbox" bind:checked={checks[f.key]} /><span>{f.label}</span></label>
          {:else}
            <label class="k-field">
              <span class="label">{f.label}</span>
              {#if f.type === "select"}
                <select class="k-input" bind:value={texts[f.key]}>
                  {#each f.options ?? [] as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
                </select>
              {:else if f.multiline}
                <textarea class="k-input" rows="4" placeholder={f.placeholder} bind:value={texts[f.key]}></textarea>
              {:else}
                <input
                  class="k-input"
                  type={f.type === "password" ? "password" : "text"}
                  placeholder={f.placeholder}
                  spellcheck="false"
                  bind:value={texts[f.key]}
                />
              {/if}
            </label>
          {/if}
        {/each}
        <div class="actions">
          <button type="button" class="k-btn ghost" onclick={() => close(null)}>Cancel</button>
          <button type="submit" class="k-btn confirm {d.danger ? 'danger' : 'primary'}">{d.confirmText ?? "OK"}</button>
        </div>
      </form>
    </div>
  </div>
{/if}

<style>
  form {
    display: contents;
  }
  .msg {
    white-space: pre-wrap;
  }
</style>
