// Auto-update via the Tauri updater plugin. Releases are signed in CI; the
// app only installs updates whose signature matches the public key in
// tauri.conf.json.
import { check } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import { app, toast, errorText } from "./state.svelte";

export async function checkForUpdate(manual: boolean): Promise<void> {
  if (import.meta.env.DEV) {
    if (manual) toast("Updates are disabled in development builds.");
    return;
  }
  try {
    const update = await check();
    if (!update) {
      if (manual) toast("You're on the latest version.", "success");
      return;
    }
    app.update = {
      version: update.version,
      notes: update.body,
      install: async () => {
        await update.downloadAndInstall();
        await relaunch();
      },
    };
  } catch (e) {
    // Linux .deb/.rpm installs can't self-update; package managers handle those.
    if (manual) toast(`Couldn't check for updates: ${errorText(e)}`, "error");
  }
}
