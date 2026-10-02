<script lang="ts">
  import type { Provider } from "../api";

  let { provider, size = 16 }: { provider: Provider; size?: number } = $props();

  const LABEL: Record<Provider, string> = { github: "GH", gitlab: "GL", bitbucket: "BB" };
</script>

<span class="logo {provider}" style="width:{size}px;height:{size}px;font-size:{size * 0.45}px" title={provider}>
  {LABEL[provider]}
</span>

<style>
  .logo {
    display: inline-grid;
    place-items: center;
    border-radius: 4px;
    font-weight: 700;
    color: #fff;
    flex-shrink: 0;
    letter-spacing: -0.02em;
  }
  .github {
    background: #24292f;
  }
  :global([data-theme="dark"]) .github {
    background: #57606a;
  }
  .gitlab {
    background: #e2432a;
  }
  .bitbucket {
    background: #2684ff;
  }
</style>
