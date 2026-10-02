// Global UI state: settings, open repository tabs, toasts, dialogs.
import { api, type RepoInfo, type Settings } from "./api";
import { basename } from "./util";

export type View = "welcome" | "repo" | "settings";

export interface Toast {
  id: number;
  kind: "info" | "error" | "success";
  text: string;
}

export interface DialogField {
  key: string;
  label: string;
  value?: string;
  placeholder?: string;
  multiline?: boolean;
  type?: "text" | "checkbox" | "password" | "select";
  options?: { value: string; label: string }[];
  checked?: boolean;
}

export interface DialogRequest {
  title: string;
  message?: string;
  fields?: DialogField[];
  confirmText?: string;
  danger?: boolean;
  resolve: (v: Record<string, string | boolean> | null) => void;
}

export const app = $state({
  settings: null as Settings | null,
  view: "welcome" as View,
  tabs: [] as RepoInfo[],
  active: null as string | null,
  toasts: [] as Toast[],
  dialog: null as DialogRequest | null,
  /** Label of the network/long operation currently running, if any. */
  busy: null as string | null,
  progress: "",
  gitVersion: null as string | null,
  update: null as { version: string; notes?: string; install: () => Promise<void> } | null,
  /** Bumped to make the active repo view reload everything. */
  refreshTick: 0,
});

let toastId = 0;
export function toast(text: string, kind: Toast["kind"] = "info") {
  const id = ++toastId;
  app.toasts.push({ id, kind, text });
  setTimeout(() => dismissToast(id), kind === "error" ? 9000 : 3500);
}
export function dismissToast(id: number) {
  app.toasts = app.toasts.filter((t) => t.id !== id);
}

export function errorText(e: unknown): string {
  if (typeof e === "string") return e;
  if (e instanceof Error) return e.message;
  return JSON.stringify(e);
}

/**
 * Run an action, surfacing failures as toasts. `label` (if given) shows a
 * busy indicator; on success `done` is toasted and the repo view refreshes.
 */
export async function run<T>(fn: () => Promise<T>, opts: { label?: string; done?: string } = {}): Promise<T | undefined> {
  if (opts.label) {
    app.busy = opts.label;
    app.progress = "";
  }
  try {
    const r = await fn();
    if (opts.done) toast(opts.done, "success");
    return r;
  } catch (e) {
    toast(errorText(e), "error");
    return undefined;
  } finally {
    if (opts.label) {
      app.busy = null;
      app.progress = "";
    }
    app.refreshTick++;
  }
}

/** Like `run`, but resolves to whether the action succeeded. */
export async function runOk(fn: () => Promise<unknown>, opts: { label?: string; done?: string } = {}): Promise<boolean> {
  let ok = false;
  await run(async () => {
    await fn();
    ok = true;
  }, opts);
  return ok;
}

export function ask(req: Omit<DialogRequest, "resolve">): Promise<Record<string, string | boolean> | null> {
  return new Promise((resolve) => {
    app.dialog = { ...req, resolve };
  });
}

export async function confirm(title: string, message: string, confirmText = "OK", danger = false): Promise<boolean> {
  return (await ask({ title, message, confirmText, danger })) !== null;
}

export async function prompt(title: string, label: string, value = "", confirmText = "OK"): Promise<string | null> {
  const r = await ask({ title, confirmText, fields: [{ key: "v", label, value }] });
  const v = r?.v;
  return typeof v === "string" && v.trim() ? v.trim() : null;
}

// ---------------------------------------------------------------- settings & tabs

export async function loadSettings() {
  app.settings = await api.settingsGet();
  applyTheme();
}

export async function updateSettings(patch: Partial<Settings>) {
  app.settings = await api.settingsUpdate(patch);
  applyTheme();
}

export function applyTheme() {
  const t = app.settings?.theme ?? "system";
  const dark = t === "dark" || (t === "system" && matchMedia("(prefers-color-scheme: dark)").matches);
  document.documentElement.dataset.theme = dark ? "dark" : "light";
}

function persistTabs() {
  void api.settingsUpdate({ openRepos: app.tabs.map((t) => t.path), activeRepo: app.active ?? "" }).then((s) => {
    app.settings = s;
  });
}

export async function openRepo(path: string) {
  const info = await run(() => api.repoOpen(path));
  if (!info) return;
  addTab(info);
}

export function addTab(info: RepoInfo) {
  if (!app.tabs.some((t) => t.path === info.path)) app.tabs.push(info);
  app.active = info.path;
  app.view = "repo";
  persistTabs();
  void api.settingsGet().then((s) => (app.settings = s));
}

export function selectTab(path: string) {
  app.active = path;
  app.view = "repo";
  persistTabs();
}

export function closeTab(path: string) {
  const i = app.tabs.findIndex((t) => t.path === path);
  if (i === -1) return;
  app.tabs.splice(i, 1);
  if (app.active === path) {
    const next = app.tabs[Math.min(i, app.tabs.length - 1)];
    app.active = next?.path ?? null;
    if (!next) app.view = "welcome";
  }
  persistTabs();
}

/** Reopen the tabs from last session, silently skipping repos that vanished. */
export async function restoreTabs() {
  const s = app.settings;
  if (!s) return;
  for (const p of s.openRepos) {
    try {
      const info = await api.repoOpen(p);
      if (!app.tabs.some((t) => t.path === info.path)) app.tabs.push(info);
    } catch {
      /* repo moved or deleted */
    }
  }
  const active = app.tabs.find((t) => t.path === s.activeRepo) ?? app.tabs[0];
  if (active) {
    app.active = active.path;
    app.view = "repo";
  }
}

export function tabName(path: string) {
  return app.tabs.find((t) => t.path === path)?.name ?? basename(path);
}
