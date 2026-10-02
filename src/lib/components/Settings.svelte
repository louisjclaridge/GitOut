<script lang="ts">
  import { onMount } from "svelte";
  import { getVersion } from "@tauri-apps/api/app";
  import { api, type Account, type Provider } from "../api";
  import { app, confirm, run, updateSettings } from "../state.svelte";
  import { checkForUpdate } from "../updater";
  import Icon from "./Icon.svelte";
  import ProviderLogo from "./ProviderLogo.svelte";
  import SignIn from "./SignIn.svelte";

  let signingIn = $state<Provider | null>(null);
  let version = $state("");
  let checking = $state(false);
  let ghId = $state(app.settings?.oauth.githubClientId ?? "");
  let glId = $state(app.settings?.oauth.gitlabClientId ?? "");

  onMount(async () => {
    version = await getVersion().catch(() => "");
  });

  async function signOut(a: Account) {
    if (!(await confirm("Sign out?", `Remove ${a.username} on ${a.host} from GitOut? The saved token will be deleted from this computer.`, "Sign out", true))) return;
    const s = await run(() => api.authSignOut(a.id));
    if (s) app.settings = s;
  }

  async function saveOAuth() {
    await run(() => updateSettings({ oauth: { githubClientId: ghId.trim() || null, gitlabClientId: glId.trim() || null } }), {
      done: "Saved",
    });
  }
</script>

<div class="settings">
  <div class="inner">
    <h1>Settings</h1>

    <section>
      <h2>Accounts</h2>
      <p class="muted">
        Signed-in accounts let you browse and clone your repositories, and push or pull over HTTPS without typing
        passwords. Tokens are kept in your system keychain.
      </p>
      {#each app.settings?.accounts ?? [] as a (a.id)}
        <div class="account">
          {#if a.avatarUrl}
            <img src={a.avatarUrl} alt="" width="32" height="32" />
          {:else}
            <ProviderLogo provider={a.provider} size={32} />
          {/if}
          <div class="who">
            <strong>{a.displayName || a.username}</strong>
            <span class="muted">@{a.username} · {a.host}</span>
          </div>
          <button class="btn small" onclick={() => signOut(a)}>Sign out</button>
        </div>
      {/each}
      <div class="add">
        <button class="btn" onclick={() => (signingIn = "github")}><ProviderLogo provider="github" /> Add GitHub</button>
        <button class="btn" onclick={() => (signingIn = "gitlab")}><ProviderLogo provider="gitlab" /> Add GitLab</button>
        <button class="btn" onclick={() => (signingIn = "bitbucket")}><ProviderLogo provider="bitbucket" /> Add Bitbucket</button>
      </div>
    </section>

    <section>
      <h2>Preferences</h2>
      <label class="row">
        <span>Theme</span>
        <select value={app.settings?.theme} onchange={(e) => updateSettings({ theme: e.currentTarget.value as "system" })}>
          <option value="system">Match system</option>
          <option value="dark">Dark</option>
          <option value="light">Light</option>
        </select>
      </label>
      <label class="row">
        <span>When pulling</span>
        <select value={app.settings?.pullMode} onchange={(e) => updateSettings({ pullMode: e.currentTarget.value as "merge" })}>
          <option value="merge">Merge remote changes (default)</option>
          <option value="rebase">Rebase my commits on top</option>
          <option value="ff-only">Only fast-forward (fail if diverged)</option>
        </select>
      </label>
    </section>

    <section>
      <h2>Updates</h2>
      <label class="row check">
        <input
          type="checkbox"
          checked={app.settings?.autoUpdate}
          onchange={(e) => updateSettings({ autoUpdate: e.currentTarget.checked })}
        />
        <span>Check for updates automatically on startup</span>
      </label>
      <div class="row">
        <span class="muted">GitOut {version} · {app.gitVersion ?? "git not found"}</span>
        <button
          class="btn small"
          disabled={checking}
          onclick={async () => {
            checking = true;
            await checkForUpdate(true);
            checking = false;
          }}><Icon name="fetch" size={13} /> {checking ? "Checking…" : "Check now"}</button
        >
      </div>
    </section>

    <details>
      <summary><h2>Advanced</h2></summary>
      <p class="muted">
        Official builds include OAuth app IDs for browser sign-in. If you build GitOut yourself, register an OAuth app
        (GitHub: enable “Device Flow”; GitLab: non-confidential, with the api, read_user and write_repository scopes) and paste its
        client ID here.
      </p>
      <label class="field"><span>GitHub OAuth client ID</span><input bind:value={ghId} spellcheck="false" /></label>
      <label class="field"><span>GitLab application ID</span><input bind:value={glId} spellcheck="false" /></label>
      <button class="btn small" onclick={saveOAuth}>Save</button>
    </details>
  </div>
</div>

{#if signingIn}
  <SignIn provider={signingIn} onclose={() => (signingIn = null)} />
{/if}

<style>
  .settings {
    height: 100%;
    overflow: auto;
    display: flex;
    justify-content: center;
    padding: 36px 24px;
  }
  .inner {
    width: min(680px, 100%);
    display: flex;
    flex-direction: column;
    gap: 26px;
  }
  h1 {
    margin: 0;
    font-size: 22px;
  }
  h2 {
    margin: 0;
    font-size: 14px;
    display: inline;
  }
  section,
  details {
    display: flex;
    flex-direction: column;
    gap: 12px;
    padding: 18px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 10px;
  }
  details[open] > summary {
    margin-bottom: 12px;
  }
  details > :not(summary) {
    margin-bottom: 12px;
  }
  summary {
    cursor: pointer;
  }
  p {
    margin: 0;
  }
  .account {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 0;
    border-top: 1px solid var(--border);
  }
  .account img {
    border-radius: 50%;
  }
  .who {
    flex: 1;
    display: flex;
    flex-direction: column;
    min-width: 0;
  }
  .add {
    display: flex;
    gap: 8px;
    flex-wrap: wrap;
  }
  .row {
    display: flex;
    align-items: center;
    justify-content: space-between;
    gap: 16px;
  }
  .row select {
    width: 300px;
  }
  .row.check {
    justify-content: flex-start;
    gap: 9px;
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
</style>
