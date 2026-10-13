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

  const THEMES = [
    { value: "system", label: "System" },
    { value: "light", label: "Light" },
    { value: "dark", label: "Dark" },
  ] as const;

  async function saveOAuth() {
    await run(() => updateSettings({ oauth: { githubClientId: ghId.trim() || null, gitlabClientId: glId.trim() || null } }), {
      done: "Saved",
    });
  }
</script>

<div class="settings">
  <div class="inner">
    <h1>Settings</h1>

    <section class="k-group-section">
      <h2 class="k-group-header"><Icon name="user" /> Accounts</h2>
      <div class="k-group">
        {#each app.settings?.accounts ?? [] as a (a.id)}
          <div class="k-group-row">
            {#if a.avatarUrl}
              <img class="avatar" src={a.avatarUrl} alt="" width="32" height="32" />
            {:else}
              <ProviderLogo provider={a.provider} size={32} />
            {/if}
            <span class="label">
              <span class="ellipsis">{a.displayName || a.username}</span>
              <span class="desc ellipsis">@{a.username} · {a.host}</span>
            </span>
            <button class="k-btn" onclick={() => signOut(a)}>Sign out…</button>
          </div>
        {/each}
        <div class="k-group-row add">
          <span class="label">
            <span>Add an account</span>
            <span class="desc">Tokens are kept in your system keychain.</span>
          </span>
          <button class="k-btn" onclick={() => (signingIn = "github")}><ProviderLogo provider="github" /> GitHub</button>
          <button class="k-btn" onclick={() => (signingIn = "gitlab")}><ProviderLogo provider="gitlab" /> GitLab</button>
          <button class="k-btn" onclick={() => (signingIn = "bitbucket")}><ProviderLogo provider="bitbucket" /> Bitbucket</button>
        </div>
      </div>
      <p class="k-group-footer">
        Signed-in accounts let you browse and clone your repositories, and push or pull over HTTPS without typing passwords.
      </p>
    </section>

    <section class="k-group-section">
      <h2 class="k-group-header"><Icon name="palette" /> Appearance</h2>
      <div class="k-group">
        <div class="k-group-row">
          <span class="label">Theme</span>
          <div class="k-tabs" role="tablist" aria-label="Theme">
            {#each THEMES as t (t.value)}
              <button
                class="k-tab"
                role="tab"
                aria-selected={(app.settings?.theme ?? "system") === t.value}
                onclick={() => updateSettings({ theme: t.value })}>{t.label}</button
              >
            {/each}
          </div>
        </div>
      </div>
      <p class="k-group-footer">System follows your OS setting.</p>
    </section>

    <section class="k-group-section">
      <h2 class="k-group-header"><Icon name="pull" /> Pulling</h2>
      <div class="k-group">
        <label class="k-group-row">
          <span class="label">When pulling</span>
          <select
            class="k-input pull-mode"
            value={app.settings?.pullMode}
            onchange={(e) => updateSettings({ pullMode: e.currentTarget.value as "merge" })}
          >
            <option value="merge">Merge remote changes (default)</option>
            <option value="rebase">Rebase my commits on top</option>
            <option value="ff-only">Only fast-forward (fail if diverged)</option>
          </select>
        </label>
      </div>
    </section>

    <section class="k-group-section">
      <h2 class="k-group-header"><Icon name="download" /> Updates</h2>
      <div class="k-group">
        <label class="k-group-row switch-row">
          <span class="label">Check for updates on startup</span>
          <span class="k-switch">
            <input
              type="checkbox"
              role="switch"
              checked={app.settings?.autoUpdate}
              onchange={(e) => updateSettings({ autoUpdate: e.currentTarget.checked })}
            />
          </span>
        </label>
        <div class="k-group-row">
          <span class="label">
            <span>GitOut {version}</span>
            <span class="desc">{app.gitVersion ?? "git not found"}</span>
          </span>
          <button
            class="k-btn"
            disabled={checking}
            onclick={async () => {
              checking = true;
              await checkForUpdate(true);
              checking = false;
            }}
            >{#if checking}<span class="k-spinner"></span> Checking…{:else}<Icon name="refresh" /> Check now{/if}</button
          >
        </div>
      </div>
    </section>

    <details class="k-group-section advanced">
      <summary class="k-group-header">
        <Icon name="sliders" /> Advanced
        <span class="chev"><Icon name="chevron" size={14} /></span>
      </summary>
      <div class="k-group">
        <div class="k-group-row stack">
          <p class="desc">
            Official builds include OAuth app IDs for browser sign-in. If you build GitOut yourself, register an OAuth app
            (GitHub: enable “Device Flow”; GitLab: non-confidential, with the api, read_user and write_repository scopes) and
            paste its client ID here.
          </p>
          <label class="k-field">
            <span class="label">GitHub OAuth client ID</span>
            <input class="k-input mono" bind:value={ghId} spellcheck="false" />
          </label>
          <label class="k-field">
            <span class="label">GitLab application ID</span>
            <input class="k-input mono" bind:value={glId} spellcheck="false" />
          </label>
          <button class="k-btn save" onclick={saveOAuth}>Save</button>
        </div>
      </div>
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
    padding: var(--space-32) var(--space-24);
    background: var(--bg);
  }
  .inner {
    width: min(680px, 100%);
    display: flex;
    flex-direction: column;
  }
  h1 {
    font-size: 20px;
    line-height: 26px;
    font-weight: 600;
    letter-spacing: -0.01em;
    margin-bottom: var(--space-24);
  }
  .avatar {
    border-radius: var(--radius-full);
    flex: none;
  }
  .add {
    flex-wrap: wrap;
    gap: var(--space-8);
  }
  .add .label {
    min-width: 180px;
  }
  .pull-mode {
    width: 280px;
  }
  .switch-row {
    cursor: pointer;
  }
  .stack {
    gap: var(--space-12);
  }
  .stack .desc {
    font-size: 12px;
    line-height: 16px;
    color: var(--text-3);
  }
  .save {
    align-self: flex-start;
  }
  summary {
    cursor: pointer;
    list-style: none;
  }
  summary::-webkit-details-marker {
    display: none;
  }
  .chev {
    display: inline-flex;
    color: var(--text-3);
    transition: transform 120ms ease;
  }
  .advanced[open] .chev {
    transform: rotate(90deg);
  }
</style>
