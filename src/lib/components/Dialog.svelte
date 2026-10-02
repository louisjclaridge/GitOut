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
  <div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && close(null)}>
    <form
      class="dialog"
      bind:this={form}
      onsubmit={(e) => {
        e.preventDefault();
        close({ ...texts, ...checks });
      }}
    >
      <h3>{d.title}</h3>
      {#if d.message}<p class="msg">{d.message}</p>{/if}
      {#each d.fields ?? [] as f (f.key)}
        {#if f.type === "checkbox"}
          <label class="check"><input type="checkbox" bind:checked={checks[f.key]} /> {f.label}</label>
        {:else}
          <label class="field">
            <span>{f.label}</span>
            {#if f.type === "select"}
              <select bind:value={texts[f.key]}>
                {#each f.options ?? [] as o (o.value)}<option value={o.value}>{o.label}</option>{/each}
              </select>
            {:else if f.multiline}
              <textarea rows="4" placeholder={f.placeholder} bind:value={texts[f.key]}></textarea>
            {:else}
              <input
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
        <button type="button" class="btn" onclick={() => close(null)}>Cancel</button>
        <button type="submit" class="btn confirm {d.danger ? 'danger' : 'primary'}">{d.confirmText ?? "OK"}</button>
      </div>
    </form>
  </div>
{/if}

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 900;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.45);
  }
  .dialog {
    width: min(460px, calc(100vw - 32px));
    padding: 20px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
    display: flex;
    flex-direction: column;
    gap: 12px;
  }
  h3 {
    margin: 0;
    font-size: 15px;
  }
  .msg {
    margin: 0;
    color: var(--muted);
    white-space: pre-wrap;
    user-select: text;
  }
  .field {
    display: flex;
    flex-direction: column;
    gap: 5px;
  }
  .field span {
    font-size: 12px;
    color: var(--muted);
  }
  .check {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .actions {
    display: flex;
    justify-content: flex-end;
    gap: 8px;
    margin-top: 4px;
  }
</style>
