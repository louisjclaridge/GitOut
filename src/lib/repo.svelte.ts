// State and actions for one open repository.
import { openPath } from "@tauri-apps/plugin-opener";
import { api, type Commit, type DiffMode, type Ref, type Stash, type Status } from "./api";
import { layoutGraph } from "./graph";
import { app, ask, confirm, prompt, run, runOk, toast } from "./state.svelte";
import type { MenuItem } from "./menu.svelte";
import { shortHash } from "./util";

export type Selection = { kind: "wip" } | { kind: "commit"; hash: string } | null;

export interface DiffTarget {
  file: string;
  orig: string | null;
  mode: DiffMode;
  hash?: string;
}

const PAGE = 2000;

export class RepoStore {
  path: string;
  status = $state<Status | null>(null);
  commits = $state<Commit[]>([]);
  refs = $state<Ref[]>([]);
  stashes = $state<Stash[]>([]);
  selected = $state<Selection>(null);
  diff = $state<DiffTarget | null>(null);
  limit = $state(PAGE);
  loaded = $state(false);
  /** Request the graph to scroll a commit into view. */
  scrollTo = $state<string | null>(null);

  layout = $derived(layoutGraph(this.commits));
  dirty = $derived((this.status?.files.length ?? 0) > 0);
  hasMore = $derived(this.commits.length >= this.limit);
  headHash = $derived(this.status?.head ?? null);
  currentBranch = $derived(this.status?.branch ?? null);

  constructor(path: string) {
    this.path = path;
  }

  #loading: Promise<void> | null = null;
  #again = false;

  /** Reload everything. Calls made while a load is running coalesce into one more. */
  async load(): Promise<void> {
    if (this.#loading) {
      this.#again = true;
      return this.#loading;
    }
    this.#loading = (async () => {
      do {
        this.#again = false;
        try {
          const [status, commits, refs, stashes] = await Promise.all([
            api.status(this.path),
            api.log(this.path, this.limit),
            api.refs(this.path),
            api.stashes(this.path),
          ]);
          this.status = status;
          this.commits = commits;
          this.refs = refs;
          this.stashes = stashes;
          if (!this.loaded) {
            this.selected = status.files.length ? { kind: "wip" } : commits[0] ? { kind: "commit", hash: commits[0].hash } : null;
          } else if (this.selected?.kind === "wip" && !status.files.length && !status.state) {
            this.selected = commits[0] ? { kind: "commit", hash: commits[0].hash } : null;
          }
          this.loaded = true;
        } catch (e) {
          toast(String(e), "error");
        }
      } while (this.#again);
      this.#loading = null;
    })();
    return this.#loading;
  }

  async loadMore() {
    this.limit += PAGE;
    await this.load();
  }

  select(sel: Selection) {
    this.selected = sel;
    this.diff = null;
  }

  selectCommit(hash: string) {
    this.select({ kind: "commit", hash });
    this.scrollTo = hash;
  }

  // ------------------------------------------------------------ sync

  fetch() {
    return run(() => api.fetch(this.path), { label: "Fetching", done: "Fetched from all remotes" });
  }
  pull() {
    return run(() => api.pull(this.path), { label: "Pulling", done: "Pulled" });
  }
  async push(force = false) {
    if (force && !(await confirm("Force push?", "This overwrites the remote branch with your local history (using --force-with-lease, so it fails safely if someone else pushed in the meantime).", "Force push", true))) return;
    const r = await run(() => api.push(this.path, force), { label: "Pushing", done: "Pushed" });
    return r;
  }

  // ------------------------------------------------------------ branches

  async createBranch(start: string | null = null) {
    const r = await ask({
      title: start ? `New branch at ${shortHash(start)}` : "New branch",
      confirmText: "Create branch",
      fields: [
        { key: "name", label: "Branch name", placeholder: "feature/my-change" },
        { key: "checkout", label: "Switch to the new branch", type: "checkbox", checked: true },
      ],
    });
    const name = typeof r?.name === "string" ? r.name.trim().replace(/\s+/g, "-") : "";
    if (!name) return;
    await run(() => api.branchCreate(this.path, name, start, r?.checkout === true), { done: `Created ${name}` });
  }

  checkout(ref: Ref) {
    if (ref.kind === "local") {
      if (ref.current) return;
      return run(() => api.checkout(this.path, ref.short, "local"), { done: `Switched to ${ref.short}` });
    }
    if (ref.kind === "remote") {
      return run(() => api.checkout(this.path, ref.short, "remote"), { done: `Switched to ${ref.short.split("/").slice(1).join("/")}` });
    }
    return this.checkoutCommit(ref.target);
  }

  async checkoutCommit(hash: string) {
    if (!(await confirm("Check out commit?", `You'll be in "detached HEAD" state at ${shortHash(hash)}. Create a branch if you want to keep new commits.`, "Check out"))) return;
    await run(() => api.checkout(this.path, hash, "commit"), { done: `Checked out ${shortHash(hash)}` });
  }

  async renameBranch(ref: Ref) {
    const name = await prompt("Rename branch", "New name", ref.short, "Rename");
    if (!name || name === ref.short) return;
    await run(() => api.branchRename(this.path, ref.short, name), { done: `Renamed to ${name}` });
  }

  async deleteBranch(ref: Ref) {
    if (ref.kind === "remote") {
      if (!(await confirm("Delete remote branch?", `This deletes ${ref.short} on the server for everyone.`, "Delete", true))) return;
      await run(() => api.remoteBranchDelete(this.path, ref.short), { label: `Deleting ${ref.short}`, done: `Deleted ${ref.short}` });
      return;
    }
    if (!(await confirm("Delete branch?", `Delete local branch ${ref.short}?`, "Delete", true))) return;
    const ok = await runOk(() => api.branchDelete(this.path, ref.short, false), { done: `Deleted ${ref.short}` });
    if (!ok) {
      // Usually "not fully merged" — offer to force it.
      if (await confirm("Branch isn't merged", `${ref.short} has commits that aren't merged anywhere. Delete it anyway? Those commits may be lost.`, "Force delete", true)) {
        await run(() => api.branchDelete(this.path, ref.short, true), { done: `Deleted ${ref.short}` });
      }
    }
  }

  async merge(target: string) {
    const into = this.currentBranch ?? "HEAD";
    if (!(await confirm("Merge", `Merge ${target} into ${into}?`, "Merge"))) return;
    await run(() => api.merge(this.path, target), { done: `Merged ${target} into ${into}` });
  }

  async rebase(onto: string) {
    const cur = this.currentBranch ?? "HEAD";
    if (!(await confirm("Rebase", `Rebase ${cur} onto ${onto}? This rewrites ${cur}'s commits.`, "Rebase"))) return;
    await run(() => api.rebase(this.path, onto), { done: `Rebased ${cur} onto ${onto}` });
  }

  // ------------------------------------------------------------ commits

  async reset(hash: string) {
    const branch = this.currentBranch ?? "HEAD";
    const r = await ask({
      title: `Reset ${branch} to ${shortHash(hash)}`,
      message: "Moves the branch to this commit. Choose what happens to the changes in the commits after it.",
      confirmText: "Reset",
      danger: true,
      fields: [
        {
          key: "mode",
          label: "Mode",
          type: "select",
          value: "mixed",
          options: [
            { value: "soft", label: "Soft — keep changes staged" },
            { value: "mixed", label: "Mixed — keep changes as unstaged" },
            { value: "hard", label: "Hard — discard all changes (cannot be undone)" },
          ],
        },
      ],
    });
    if (!r) return;
    const mode = r.mode as "soft" | "mixed" | "hard";
    await run(() => api.reset(this.path, hash, mode), { done: `Reset ${branch} to ${shortHash(hash)}` });
  }

  async createTag(hash: string) {
    const r = await ask({
      title: `Tag ${shortHash(hash)}`,
      confirmText: "Create tag",
      fields: [
        { key: "name", label: "Tag name", placeholder: "v1.0.0" },
        { key: "message", label: "Message (optional, makes an annotated tag)", multiline: true },
        { key: "push", label: "Push tag to remote", type: "checkbox", checked: false },
      ],
    });
    const name = typeof r?.name === "string" ? r.name.trim() : "";
    if (!name) return;
    const ok = await runOk(() => api.tagCreate(this.path, name, hash, (r?.message as string) || null), { done: `Tagged ${name}` });
    if (ok && r?.push) await run(() => api.pushTag(this.path, name), { label: `Pushing ${name}`, done: `Pushed ${name}` });
  }

  commitMenu(c: Commit): MenuItem[] {
    const isHead = c.hash === this.headHash;
    return [
      { label: "Create branch here…", icon: "branch", action: () => this.createBranch(c.hash) },
      { label: "Create tag here…", icon: "tag", action: () => this.createTag(c.hash) },
      "sep",
      { label: "Check out this commit", icon: "commit", action: () => this.checkoutCommit(c.hash), disabled: isHead },
      { label: "Cherry-pick onto current branch", icon: "plus", action: () => run(() => api.cherryPick(this.path, c.hash), { done: "Cherry-picked" }), disabled: isHead },
      { label: "Revert this commit", icon: "undo", action: () => run(() => api.revert(this.path, c.hash), { done: "Reverted" }) },
      { label: `Reset ${this.currentBranch ?? "HEAD"} to here…`, icon: "warning", danger: true, action: () => this.reset(c.hash), disabled: isHead },
      "sep",
      { label: "Copy commit hash", icon: "copy", action: () => copy(c.hash, "Hash copied") },
      { label: "Copy message", icon: "copy", action: () => copy(c.subject, "Message copied") },
    ];
  }

  refMenu(ref: Ref): MenuItem[] {
    const cur = this.currentBranch;
    const items: MenuItem[] = [];
    if (ref.kind === "local") {
      items.push({ label: ref.current ? "Current branch" : `Switch to ${ref.short}`, icon: "check", action: () => this.checkout(ref), disabled: ref.current });
    } else if (ref.kind === "remote") {
      items.push({ label: `Check out ${ref.short}`, icon: "check", action: () => this.checkout(ref) });
    } else {
      items.push({ label: "Check out tag", icon: "check", action: () => this.checkoutCommit(ref.target) });
    }
    if (!ref.current && cur) {
      items.push(
        { label: `Merge ${ref.short} into ${cur}`, icon: "branch", action: () => this.merge(ref.short) },
        { label: `Rebase ${cur} onto ${ref.short}`, icon: "branch", action: () => this.rebase(ref.short) },
      );
    }
    items.push("sep", { label: "Create branch here…", icon: "plus", action: () => this.createBranch(ref.short) });
    if (ref.kind === "local") items.push({ label: "Rename…", icon: "commit", action: () => this.renameBranch(ref) });
    if (ref.kind === "tag") {
      items.push(
        { label: "Push tag", icon: "push", action: () => run(() => api.pushTag(this.path, ref.short), { label: `Pushing ${ref.short}`, done: `Pushed ${ref.short}` }) },
        "sep",
        { label: "Delete tag", icon: "x", danger: true, action: async () => (await confirm("Delete tag?", `Delete local tag ${ref.short}?`, "Delete", true)) && run(() => api.tagDelete(this.path, ref.short), { done: `Deleted ${ref.short}` }) },
      );
    } else {
      items.push("sep", { label: ref.kind === "remote" ? "Delete from remote…" : "Delete…", icon: "x", danger: true, action: () => this.deleteBranch(ref), disabled: ref.current });
    }
    items.push("sep", { label: "Copy name", icon: "copy", action: () => copy(ref.short, "Copied") });
    return items;
  }

  // ------------------------------------------------------------ stash

  async stash() {
    const r = await ask({
      title: "Stash changes",
      confirmText: "Stash",
      fields: [
        { key: "message", label: "Message (optional)" },
        { key: "untracked", label: "Include new (untracked) files", type: "checkbox", checked: true },
      ],
    });
    if (!r) return;
    await run(() => api.stashSave(this.path, (r.message as string) || null, r.untracked === true), { done: "Changes stashed" });
  }

  popLatest() {
    if (!this.stashes.length) return;
    return run(() => api.stashAction(this.path, 0, "pop"), { done: "Stash popped" });
  }

  stashMenu(s: Stash): MenuItem[] {
    return [
      { label: "Apply", icon: "pop", action: () => run(() => api.stashAction(this.path, s.index, "apply"), { done: "Stash applied" }) },
      { label: "Pop (apply and remove)", icon: "pop", action: () => run(() => api.stashAction(this.path, s.index, "pop"), { done: "Stash popped" }) },
      "sep",
      { label: "Delete", icon: "x", danger: true, action: async () => (await confirm("Delete stash?", s.message, "Delete", true)) && run(() => api.stashAction(this.path, s.index, "drop"), { done: "Stash deleted" }) },
    ];
  }

  openFolder() {
    return run(() => openPath(this.path));
  }
}

async function copy(text: string, msg: string) {
  try {
    await navigator.clipboard.writeText(text);
    toast(msg);
  } catch {
    toast("Couldn't access the clipboard", "error");
  }
}

/** Whether a long-running network op is in progress (disables toolbar buttons). */
export function isBusy() {
  return app.busy !== null;
}
