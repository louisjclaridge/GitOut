// "Fetch & pull all": brings every open repository up to date, one at a time.
import { api } from "./api";
import { app, errorText, toast } from "./state.svelte";

export type SyncStep = "waiting" | "fetching" | "pulling" | "done" | "skipped" | "error";

export interface SyncItem {
  step: SyncStep;
  /** What happened, or why it failed. */
  detail: string;
}

export const sync = $state({
  running: false,
  /** Per repository path, from the latest run. */
  items: {} as Record<string, SyncItem>,
});

/**
 * Fetch every open repository, then pull the ones whose branch is behind its
 * upstream. Repositories on a detached HEAD or an unpublished branch are only
 * fetched. One at a time, so git's progress and credential prompts don't interleave.
 */
export async function syncAll() {
  if (sync.running || app.busy) return;
  const tabs = [...app.tabs];
  if (!tabs.length) return;
  sync.running = true;
  sync.items = Object.fromEntries(tabs.map((t) => [t.path, { step: "waiting", detail: "" }]));
  const set = (path: string, step: SyncStep, detail = "") => (sync.items[path] = { step, detail });
  let pulled = 0;
  const failed: string[] = [];

  try {
    for (const [i, t] of tabs.entries()) {
      const of = tabs.length > 1 ? ` (${i + 1} of ${tabs.length})` : "";
      try {
        app.busy = `Fetching ${t.name}${of}`;
        app.progress = "";
        set(t.path, "fetching");
        await api.fetch(t.path);
        const st = await api.status(t.path);
        if (!st.branch) {
          set(t.path, "skipped", "Fetched. Not on a branch, so nothing to pull.");
        } else if (!st.upstream) {
          set(t.path, "skipped", `Fetched. ${st.branch} isn't tracking a remote branch.`);
        } else if (!st.behind) {
          set(t.path, "done", "Up to date");
        } else {
          app.busy = `Pulling ${t.name}${of}`;
          app.progress = "";
          set(t.path, "pulling");
          await api.pull(t.path);
          set(t.path, "done", `Pulled ${st.behind} commit${st.behind === 1 ? "" : "s"} into ${st.branch}`);
          pulled++;
        }
      } catch (e) {
        set(t.path, "error", errorText(e));
        failed.push(t.name);
      }
    }
  } finally {
    app.busy = null;
    app.progress = "";
    sync.running = false;
    app.refreshTick++;
  }

  const n = tabs.length;
  const repos = `${n} repositor${n === 1 ? "y" : "ies"}`;
  if (failed.length) {
    toast(`Couldn't sync ${failed.join(", ")}. See the home page for details.`, "error");
  } else {
    toast(pulled ? `Synced ${repos}, pulled into ${pulled}` : `Fetched ${repos}, all up to date`, "success");
  }
}
