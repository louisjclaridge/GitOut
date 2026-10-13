<script lang="ts">
  import { onDestroy } from "svelte";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import { api, type DeviceStart, type Provider } from "../api";
  import { errorText, loadSettings, toast } from "../state.svelte";
  import Icon from "./Icon.svelte";
  import ProviderLogo from "./ProviderLogo.svelte";

  let { provider, onclose }: { provider: Provider; onclose: () => void } = $props();

  const NAMES: Record<Provider, string> = { github: "GitHub", gitlab: "GitLab", bitbucket: "Bitbucket" };
  const TOKEN_HELP: Record<Provider, { url: string; text: string }> = {
    github: {
      url: "https://github.com/settings/tokens/new?scopes=repo,read:user,workflow&description=GitOut",
      text: "Create a classic token with the repo, read:user and workflow scopes.",
    },
    gitlab: {
      url: "https://gitlab.com/-/user_settings/personal_access_tokens?name=GitOut&scopes=api,read_user,write_repository",
      text: "Create a token with the api, read_user and write_repository scopes.",
    },
    bitbucket: {
      url: "https://id.atlassian.com/manage-profile/security/api-tokens",
      text: "Create an API token with scopes, including repository read and write, and enter your Atlassian account email.",
    },
  };

  // Bitbucket Cloud has no OAuth device flow, so it always uses a token.
  // svelte-ignore state_referenced_locally
  let method = $state<"browser" | "token">(provider === "bitbucket" ? "token" : "browser");
  let host = $state("");
  let token = $state("");
  let login = $state("");
  let device = $state<DeviceStart | null>(null);
  let busy = $state(false);
  let error = $state("");
  let showHost = $state(false);

  async function startBrowser() {
    error = "";
    busy = true;
    try {
      device = await api.authDeviceStart(provider, host.trim() || undefined);
      void openUrl(device.verificationUriComplete ?? device.verificationUri);
      const account = await api.authDevicePoll(device);
      await done(account.displayName || account.username);
    } catch (e) {
      error = errorText(e);
      device = null;
    } finally {
      busy = false;
    }
  }

  async function submitToken(e: Event) {
    e.preventDefault();
    error = "";
    busy = true;
    try {
      const account = await api.authToken(provider, host.trim() || null, token, login.trim() || null);
      await done(account.displayName || account.username);
    } catch (err) {
      error = errorText(err);
    } finally {
      busy = false;
    }
  }

  async function done(name: string) {
    await loadSettings();
    toast(`Signed in to ${NAMES[provider]} as ${name}`, "success");
    onclose();
  }

  function cancel() {
    if (device) void api.authCancel();
    onclose();
  }

  onDestroy(() => {
    if (busy && device) void api.authCancel();
  });

  function copyCode() {
    if (device) void navigator.clipboard.writeText(device.userCode).then(() => toast("Code copied"));
  }
</script>

<div class="k-scrim" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <div
    class="k-dialog"
    role="dialog"
    aria-modal="true"
    aria-labelledby="signin-title"
    tabindex="-1"
    onkeydown={(e) => e.key === "Escape" && cancel()}
  >
    <header>
      <ProviderLogo {provider} size={24} />
      <h2 id="signin-title">Sign in to {NAMES[provider]}</h2>
      <button class="k-icon-btn" aria-label="Close" title="Close" onclick={cancel}><Icon name="x" /></button>
    </header>

    {#if provider !== "bitbucket" && !device}
      <div class="k-tabs fill" role="tablist" aria-label="Sign-in method">
        <button class="k-tab" role="tab" aria-selected={method === "browser"} onclick={() => (method = "browser")}>Browser</button>
        <button class="k-tab" role="tab" aria-selected={method === "token"} onclick={() => (method = "token")}>Access token</button>
      </div>
    {/if}

    {#if provider !== "bitbucket" && !device}
      {#if showHost}
        <label class="k-field">
          <span class="label">Server</span>
          <input
            class="k-input mono"
            bind:value={host}
            placeholder={provider === "github" ? "github.mycompany.com" : "gitlab.mycompany.com"}
            spellcheck="false"
          />
          <span class="hint">Leave empty for {provider === "github" ? "github.com" : "gitlab.com"}.</span>
        </label>
      {:else}
        <button class="k-link small" onclick={() => (showHost = true)}>Using a self-hosted server?</button>
      {/if}
    {/if}

    {#if method === "browser"}
      {#if device}
        <div class="device">
          <p>Enter this code in the browser window that just opened:</p>
          <button class="code mono" title="Copy code" onclick={copyCode}>{device.userCode} <Icon name="copy" size={20} /></button>
          <button class="k-btn" onclick={() => device && openUrl(device.verificationUriComplete ?? device.verificationUri)}>
            <Icon name="external" /> Open {device.verificationUri.replace(/^https?:\/\//, "")}
          </button>
          <p class="waiting"><span class="k-spinner"></span> Waiting for you to approve…</p>
        </div>
      {:else}
        <p>You'll get a short code to enter on {NAMES[provider]}. GitOut never sees your password.</p>
        <button class="k-btn primary block" disabled={busy} onclick={startBrowser}>
          {#if busy}<span class="k-spinner"></span>{:else}<Icon name="external" />{/if} Continue in browser
        </button>
      {/if}
    {:else}
      <form onsubmit={submitToken}>
        <p>
          {TOKEN_HELP[provider].text}
          <button type="button" class="k-link" onclick={() => openUrl(TOKEN_HELP[provider].url)}>Create one</button>
        </p>
        {#if provider === "bitbucket"}
          <label class="k-field">
            <span class="label">Atlassian account email</span>
            <input class="k-input" bind:value={login} type="email" placeholder="you@example.com" spellcheck="false" />
          </label>
        {/if}
        <label class="k-field">
          <span class="label">{provider === "bitbucket" ? "API token" : "Personal access token"}</span>
          <input class="k-input mono" bind:value={token} type="password" spellcheck="false" autocomplete="off" />
        </label>
        <button class="k-btn primary block" type="submit" disabled={busy || !token.trim() || (provider === "bitbucket" && !login.trim())}>
          {#if busy}<span class="k-spinner"></span> Checking…{:else}Sign in{/if}
        </button>
      </form>
    {/if}

    {#if error}
      <div class="k-banner danger" role="alert">
        <Icon name="error" size={20} />
        <div class="body"><div class="desc selectable">{error}</div></div>
      </div>
    {/if}
  </div>
</div>

<style>
  header {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    margin: calc(-1 * var(--space-6)) calc(-1 * var(--space-8)) 0 0;
  }
  h2 {
    flex: 1;
  }
  p {
    color: var(--text-2);
  }
  form {
    display: flex;
    flex-direction: column;
    gap: var(--space-12);
  }
  .small {
    font-size: 12px;
  }
  .device {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: var(--space-12);
    text-align: center;
  }
  .code {
    display: flex;
    align-items: center;
    gap: var(--space-12);
    padding: var(--space-12) var(--space-20);
    font-size: 26px;
    line-height: 32px;
    font-weight: 700;
    letter-spacing: 0.12em;
    border: 0;
    border-radius: var(--radius-md);
    background: var(--accent-soft);
    color: var(--text);
    user-select: text;
  }
  .code :global(.k-icon) {
    color: var(--accent-text);
  }
  .waiting {
    display: flex;
    align-items: center;
    gap: var(--space-8);
    color: var(--text-3);
  }
  .k-banner .desc {
    white-space: pre-wrap;
  }
</style>
