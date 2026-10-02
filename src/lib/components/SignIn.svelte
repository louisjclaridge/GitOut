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

<div class="backdrop" role="presentation" onmousedown={(e) => e.target === e.currentTarget && cancel()}>
  <div class="dialog" role="dialog" aria-modal="true" tabindex="-1" onkeydown={(e) => e.key === "Escape" && cancel()}>
    <header>
      <ProviderLogo {provider} size={22} />
      <h3>Sign in to {NAMES[provider]}</h3>
      <button class="btn ghost icon" aria-label="Close" onclick={cancel}><Icon name="x" /></button>
    </header>

    {#if provider !== "bitbucket" && !device}
      <div class="seg">
        <button class:active={method === "browser"} onclick={() => (method = "browser")}>Browser</button>
        <button class:active={method === "token"} onclick={() => (method = "token")}>Access token</button>
      </div>
    {/if}

    {#if provider !== "bitbucket" && !device}
      {#if showHost}
        <label class="field">
          <span>Server (leave empty for {provider === "github" ? "github.com" : "gitlab.com"})</span>
          <input bind:value={host} placeholder={provider === "github" ? "github.mycompany.com" : "gitlab.mycompany.com"} spellcheck="false" />
        </label>
      {:else}
        <button class="link" onclick={() => (showHost = true)}>Using a self-hosted server?</button>
      {/if}
    {/if}

    {#if method === "browser"}
      {#if device}
        <div class="device">
          <p>Enter this code in the browser window that just opened:</p>
          <button class="code mono" title="Copy code" onclick={copyCode}>{device.userCode} <Icon name="copy" size={16} /></button>
          <button class="btn small" onclick={() => device && openUrl(device.verificationUriComplete ?? device.verificationUri)}>
            <Icon name="external" size={13} /> Open {device.verificationUri.replace(/^https?:\/\//, "")}
          </button>
          <p class="waiting muted"><span class="spinner"></span> Waiting for you to approve…</p>
        </div>
      {:else}
        <p class="muted">
          You'll get a short code to enter on {NAMES[provider]}. GitOut never sees your password.
        </p>
        <button class="btn primary big" disabled={busy} onclick={startBrowser}>
          <Icon name="external" size={15} /> Continue in browser
        </button>
      {/if}
    {:else}
      <form onsubmit={submitToken}>
        <p class="muted">
          {TOKEN_HELP[provider].text}
          <button type="button" class="link" onclick={() => openUrl(TOKEN_HELP[provider].url)}>Create one →</button>
        </p>
        {#if provider === "bitbucket"}
          <label class="field">
            <span>Atlassian account email</span>
            <input bind:value={login} type="email" placeholder="you@example.com" spellcheck="false" />
          </label>
        {/if}
        <label class="field">
          <span>{provider === "bitbucket" ? "API token" : "Personal access token"}</span>
          <input bind:value={token} type="password" spellcheck="false" autocomplete="off" />
        </label>
        <button class="btn primary big" type="submit" disabled={busy || !token.trim() || (provider === "bitbucket" && !login.trim())}>
          {busy ? "Checking…" : "Sign in"}
        </button>
      </form>
    {/if}

    {#if error}<div class="error selectable">{error}</div>{/if}
  </div>
</div>

<style>
  .backdrop {
    position: fixed;
    inset: 0;
    z-index: 800;
    display: grid;
    place-items: center;
    background: rgba(0, 0, 0, 0.45);
  }
  .dialog {
    width: min(440px, calc(100vw - 32px));
    padding: 18px 20px 20px;
    display: flex;
    flex-direction: column;
    gap: 14px;
    background: var(--panel);
    border: 1px solid var(--border);
    border-radius: 12px;
    box-shadow: var(--shadow);
  }
  header {
    display: flex;
    align-items: center;
    gap: 10px;
  }
  h3 {
    flex: 1;
    margin: 0;
    font-size: 15px;
  }
  p {
    margin: 0;
  }
  .seg {
    display: flex;
    padding: 3px;
    border-radius: 8px;
    background: var(--bg);
    border: 1px solid var(--border);
  }
  .seg button {
    flex: 1;
    padding: 6px;
    border: 0;
    border-radius: 6px;
    background: none;
    color: var(--muted);
  }
  .seg button.active {
    background: var(--panel-2);
    color: var(--text);
    box-shadow: 0 1px 2px rgba(0, 0, 0, 0.2);
  }
  form {
    display: flex;
    flex-direction: column;
    gap: 12px;
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
  .link {
    padding: 0;
    border: 0;
    background: none;
    color: var(--accent);
    text-align: left;
    font-size: 12px;
  }
  .big {
    padding: 9px;
    font-weight: 600;
  }
  .device {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 12px;
    text-align: center;
  }
  .code {
    display: flex;
    align-items: center;
    gap: 10px;
    padding: 10px 18px;
    font-size: 26px;
    font-weight: 700;
    letter-spacing: 0.12em;
    border: 1px dashed var(--accent);
    border-radius: 10px;
    background: var(--accent-soft);
    color: var(--text);
  }
  .waiting {
    display: flex;
    align-items: center;
    gap: 8px;
  }
  .spinner {
    width: 12px;
    height: 12px;
    border: 2px solid var(--accent);
    border-right-color: transparent;
    border-radius: 50%;
    animation: spin 0.7s linear infinite;
  }
  @keyframes spin {
    to {
      transform: rotate(360deg);
    }
  }
  .error {
    padding: 8px 10px;
    border-radius: 6px;
    background: var(--del-bg);
    color: var(--del-fg);
    white-space: pre-wrap;
  }
</style>
