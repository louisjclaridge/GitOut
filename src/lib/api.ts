// Typed wrappers around the Rust commands in src-tauri/src/commands.rs.
import { invoke } from "@tauri-apps/api/core";

export type Provider = "github" | "gitlab" | "bitbucket";

export interface Account {
  id: string;
  provider: Provider;
  host: string;
  username: string;
  displayName?: string | null;
  avatarUrl?: string | null;
  gitUser: string;
}

export interface Settings {
  recentRepos: string[];
  openRepos: string[];
  activeRepo: string | null;
  accounts: Account[];
  theme: "system" | "dark" | "light";
  pullMode: "merge" | "rebase" | "ff-only";
  autoUpdate: boolean;
  oauth: { githubClientId?: string | null; gitlabClientId?: string | null };
}

export interface RepoInfo {
  path: string;
  name: string;
}

export interface FileChange {
  path: string;
  origPath: string | null;
  index: string;
  worktree: string;
  conflicted: boolean;
}

export interface Status {
  branch: string | null;
  head: string | null;
  upstream: string | null;
  ahead: number;
  behind: number;
  files: FileChange[];
  state: "merging" | "rebasing" | "cherry-picking" | "reverting" | null;
}

export interface Commit {
  hash: string;
  parents: string[];
  author: string;
  email: string;
  time: number;
  refs: string[];
  subject: string;
}

export interface Ref {
  name: string;
  short: string;
  kind: "local" | "remote" | "tag";
  target: string;
  upstream: string | null;
  ahead: number;
  behind: number;
  current: boolean;
}

export interface Stash {
  index: number;
  hash: string;
  message: string;
}

export interface ChangedFile {
  path: string;
  origPath: string | null;
  status: string;
}

export interface CommitInfo {
  hash: string;
  parents: string[];
  author: string;
  email: string;
  time: number;
  committer: string;
  commitTime: number;
  message: string;
  files: ChangedFile[];
}

export interface Remote {
  name: string;
  url: string;
}

export interface DeviceStart {
  provider: Provider;
  host: string;
  deviceCode: string;
  userCode: string;
  verificationUri: string;
  verificationUriComplete: string | null;
  interval: number;
  expiresIn: number;
}

export interface RemoteRepo {
  fullName: string;
  description: string | null;
  private: boolean;
  httpsUrl: string;
  sshUrl: string | null;
}

export type DiffMode = "unstaged" | "staged" | "untracked" | "commit";

export interface ReleaseHost {
  provider: Provider | null;
  host: string;
  /** "owner/repo", or a GitLab namespace path. */
  project: string;
  /** The git remote tags are pushed to. */
  remote: string;
  accountId: string | null;
  webUrl: string | null;
  /** GitHub has drafts and pre-releases; GitLab doesn't. */
  supportsDrafts: boolean;
  /** Why releases can't be managed here, if they can't. */
  problem: string | null;
}

export interface ReleaseAsset {
  name: string;
  url: string;
  size: number | null;
  downloads: number | null;
}

export interface Release {
  /** GitHub: numeric id. GitLab: the tag name. */
  id: string;
  tag: string;
  name: string;
  body: string;
  draft: boolean;
  prerelease: boolean;
  publishedAt: string | null;
  createdAt: string | null;
  author: string | null;
  url: string | null;
  assets: ReleaseAsset[];
}

export interface ReleaseInput {
  tag: string;
  name: string;
  body: string;
  draft: boolean;
  prerelease: boolean;
}

export interface Changelog {
  previous: string | null;
  lines: string[];
}

export interface ConflictSide {
  title: string;
  detail: string;
  /** False when this side deleted the file. */
  exists: boolean;
  text: string | null;
}

export interface ConflictFile {
  /** The working copy with conflict markers; null if deleted or not UTF-8 text. */
  working: string | null;
  binary: boolean;
  hasBase: boolean;
  ours: ConflictSide;
  theirs: ConflictSide;
}

export const api = {
  settingsGet: () => invoke<Settings>("settings_get"),
  settingsUpdate: (patch: Partial<Settings>) => invoke<Settings>("settings_update", { patch }),
  gitVersion: () => invoke<string>("git_version"),

  repoOpen: (path: string) => invoke<RepoInfo>("repo_open", { path }),
  repoInit: (path: string) => invoke<RepoInfo>("repo_init", { path }),
  repoClone: (url: string, parent: string, name: string) => invoke<RepoInfo>("repo_clone", { url, parent, name }),
  remotes: (repo: string) => invoke<Remote[]>("remotes", { repo }),

  status: (repo: string) => invoke<Status>("status", { repo }),
  log: (repo: string, limit: number) => invoke<Commit[]>("log", { repo, limit }),
  refs: (repo: string) => invoke<Ref[]>("refs", { repo }),
  stashes: (repo: string) => invoke<Stash[]>("stashes", { repo }),
  commitInfo: (repo: string, hash: string) => invoke<CommitInfo>("commit_info", { repo, hash }),
  diff: (repo: string, file: string, orig: string | null, mode: DiffMode, hash?: string) =>
    invoke<string>("diff", { repo, file, orig, mode, hash }),

  stage: (repo: string, paths: string[]) => invoke<void>("stage", { repo, paths }),
  unstage: (repo: string, paths: string[]) => invoke<void>("unstage", { repo, paths }),
  discard: (repo: string, paths: string[], untracked: string[]) => invoke<void>("discard", { repo, paths, untracked }),
  applyPatch: (repo: string, patch: string, cached: boolean, reverse: boolean) =>
    invoke<void>("apply_patch", { repo, patch, cached, reverse }),
  commit: (repo: string, message: string, amend: boolean) => invoke<void>("commit", { repo, message, amend }),
  headMessage: (repo: string) => invoke<string>("head_message", { repo }),

  checkout: (repo: string, target: string, kind: "local" | "remote" | "commit") =>
    invoke<void>("checkout", { repo, target, kind }),
  branchCreate: (repo: string, name: string, start: string | null, checkout: boolean) =>
    invoke<void>("branch_create", { repo, name, start, checkout }),
  branchRename: (repo: string, old: string, newName: string) => invoke<void>("branch_rename", { repo, old, new: newName }),
  branchDelete: (repo: string, name: string, force: boolean) => invoke<void>("branch_delete", { repo, name, force }),
  remoteBranchDelete: (repo: string, remoteRef: string) => invoke<void>("remote_branch_delete", { repo, remoteRef }),
  merge: (repo: string, target: string) => invoke<void>("merge", { repo, target }),
  rebase: (repo: string, onto: string) => invoke<void>("rebase", { repo, onto }),
  cherryPick: (repo: string, hash: string) => invoke<void>("cherry_pick", { repo, hash }),
  revert: (repo: string, hash: string) => invoke<void>("revert", { repo, hash }),
  reset: (repo: string, hash: string, mode: "soft" | "mixed" | "hard") => invoke<void>("reset", { repo, hash, mode }),
  operationAbort: (repo: string) => invoke<void>("operation_abort", { repo }),
  operationContinue: (repo: string) => invoke<void>("operation_continue", { repo }),
  tagCreate: (repo: string, name: string, target: string, message: string | null) =>
    invoke<void>("tag_create", { repo, name, target, message }),
  tagDelete: (repo: string, name: string) => invoke<void>("tag_delete", { repo, name }),
  pushTag: (repo: string, name: string) => invoke<void>("push_tag", { repo, name }),
  remoteTagDelete: (repo: string, name: string) => invoke<void>("remote_tag_delete", { repo, name }),

  releaseHost: (repo: string) => invoke<ReleaseHost>("release_host", { repo }),
  releasesList: (repo: string) => invoke<Release[]>("releases_list", { repo }),
  releaseCreate: (repo: string, input: ReleaseInput) => invoke<Release>("release_create", { repo, input }),
  releaseUpdate: (repo: string, id: string, input: ReleaseInput) => invoke<Release>("release_update", { repo, id, input }),
  releaseDelete: (repo: string, id: string) => invoke<void>("release_delete", { repo, id }),
  releaseChangelog: (repo: string, target: string) => invoke<Changelog>("release_changelog", { repo, target }),

  conflictFile: (repo: string, file: string) => invoke<ConflictFile>("conflict_file", { repo, file }),
  conflictSave: (repo: string, file: string, content: string, stage: boolean) =>
    invoke<void>("conflict_save", { repo, file, content, stage }),
  conflictTake: (repo: string, file: string, side: "ours" | "theirs") => invoke<void>("conflict_take", { repo, file, side }),
  conflictRestart: (repo: string, file: string) => invoke<void>("conflict_restart", { repo, file }),

  stashSave: (repo: string, message: string | null, includeUntracked: boolean) =>
    invoke<void>("stash_save", { repo, message, includeUntracked }),
  stashAction: (repo: string, index: number, action: "apply" | "pop" | "drop") =>
    invoke<void>("stash_action", { repo, index, action }),

  fetch: (repo: string) => invoke<void>("fetch", { repo }),
  pull: (repo: string) => invoke<void>("pull", { repo }),
  push: (repo: string, force: boolean) => invoke<void>("push", { repo, force }),

  authDeviceStart: (provider: Provider, host?: string) => invoke<DeviceStart>("auth_device_start", { provider, host }),
  authDevicePoll: (start: DeviceStart) => invoke<Account>("auth_device_poll", { start }),
  authCancel: () => invoke<void>("auth_cancel"),
  authToken: (provider: Provider, host: string | null, token: string, login: string | null) =>
    invoke<Account>("auth_token", { provider, host, token, login }),
  authSignOut: (id: string) => invoke<Settings>("auth_sign_out", { id }),
  authListRepos: (id: string) => invoke<RemoteRepo[]>("auth_list_repos", { id }),
};
