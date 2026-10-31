// Development-only fake backend so the UI can be worked on in a plain
// browser (`npm run dev`, then open http://localhost:1420) without Rust.
// Loaded from main.ts only when not running inside Tauri.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { CiJob, CiRun, Commit, Ref, Release, ReleaseInput, Settings, Status } from "../lib/api";

const REPO = "/home/dev/projects/gitout-demo";
const now = Math.floor(Date.now() / 1000);
const people = [
  ["Ada Lovelace", "ada@example.com"],
  ["Linus Torvalds", "linus@example.com"],
  ["Grace Hopper", "grace@example.com"],
];

let n = 0;
const hash = () => (++n).toString(16).padStart(8, "0").repeat(5);

// Build a small history with a merged feature branch and an open one.
const commits: Commit[] = [];
function add(subject: string, parents: string[], refs: string[] = []): string {
  const h = hash();
  const [author, email] = people[commits.length % people.length];
  commits.unshift({ hash: h, parents, author, email, time: now - (60 - commits.length) * 5400, refs, subject });
  return h;
}
let main = add("Initial commit", []);
for (const s of ["Add README", "Set up project skeleton", "Add CI workflow"]) main = add(s, [main]);
let feat = main;
for (const s of ["Draft commit graph renderer", "Lane colouring", "Curved merge lines", "Virtualised rows"])
  feat = add(s, [feat]);
for (const s of ["Fix typo in README", "Bump dependencies"]) main = add(s, [main]);
main = add("Merge branch 'feature/graph'", [main, feat]);
let fix = main;
for (const s of ["Handle detached HEAD", "Show ahead/behind counts"]) main = add(s, [main]);
fix = add("Escape special characters in paths", [fix]);
main = add("Merge branch 'fix/paths'", [main, fix]);
const tagged = main;
let ui = main;
for (const s of ["Staging panel", "Hunk staging", "Commit box with amend"]) ui = add(s, [ui]);
const originMain = add("Release prep", [main]);
main = add("Update changelog", [originMain]);

// Decorate refs (newest commits are at the front).
const byHash = new Map(commits.map((c) => [c.hash, c]));
byHash.get(main)!.refs = ["HEAD", "refs/heads/main"];
byHash.get(originMain)!.refs = ["refs/remotes/origin/main"];
byHash.get(ui)!.refs = ["refs/heads/feature/staging-ui", "refs/remotes/origin/feature/staging-ui"];
byHash.get(tagged)!.refs = ["refs/tags/v0.1.0"];
commits.sort((a, b) => b.time - a.time);

const refs: Ref[] = [
  { name: "refs/heads/main", short: "main", kind: "local", target: main, upstream: "origin/main", ahead: 1, behind: 0, current: true },
  { name: "refs/heads/feature/staging-ui", short: "feature/staging-ui", kind: "local", target: ui, upstream: "origin/feature/staging-ui", ahead: 0, behind: 0, current: false },
  { name: "refs/remotes/origin/main", short: "origin/main", kind: "remote", target: originMain, upstream: null, ahead: 0, behind: 0, current: false },
  { name: "refs/remotes/origin/feature/staging-ui", short: "origin/feature/staging-ui", kind: "remote", target: ui, upstream: null, ahead: 0, behind: 0, current: false },
  { name: "refs/heads/louis/experiment-with-a-really-long-branch-name-that-keeps-going-to-test-the-sidebar-layout", short: "louis/experiment-with-a-really-long-branch-name-that-keeps-going-to-test-the-sidebar-layout", kind: "local", target: ui, upstream: null, ahead: 0, behind: 0, current: false },
  { name: "refs/tags/v0.1.0", short: "v0.1.0", kind: "tag", target: tagged, upstream: null, ahead: 0, behind: 0, current: false },
];

const status: Status = {
  branch: "main",
  head: main,
  upstream: "origin/main",
  ahead: 1,
  behind: 0,
  state: null,
  files: [
    { path: "src/lib/graph.ts", origPath: null, index: ".", worktree: "M", conflicted: false },
    { path: "src/App.svelte", origPath: null, index: "M", worktree: ".", conflicted: false },
    { path: "docs/notes.md", origPath: null, index: ".", worktree: "?", conflicted: false },
  ],
};

// Open the dev server with ?conflict to see the merge editor.
const DEMO_CONFLICT = new URLSearchParams(location.search).has("conflict");
if (DEMO_CONFLICT) {
  status.state = "merging";
  status.files.unshift(
    { path: "src/lib/util.ts", origPath: null, index: "U", worktree: "U", conflicted: true },
    { path: "docs/logo.png", origPath: null, index: "U", worktree: "U", conflicted: true },
  );
}

const CONFLICTED = `export function relativeTime(unix: number): string {
  const s = Math.floor(Date.now() / 1000) - unix;
<<<<<<< HEAD
  if (s < 60) return "just now";
  if (s < 3600) return \`\${Math.floor(s / 60)} min ago\`;
=======
  if (s < 45) return "a moment ago";
>>>>>>> feature/staging-ui
  const units: [number, string][] = [
    [60, "minute"],
    [3600, "hour"],
    [86400, "day"],
    [604800, "week"],
    [2629800, "month"],
    [31557600, "year"],
  ];
  let i = units.length - 1;
  while (i > 0 && s < units[i][0]) i--;
  const n = Math.floor(s / units[i][0]);
  return \`\${n} \${units[i][1]}\${n === 1 ? "" : "s"} ago\`;
}

export function basename(p: string): string {
<<<<<<< HEAD
  return p.split(/[\\/]/).filter(Boolean).pop() ?? p;
||||||| base
  return p.split("/").pop() ?? p;
=======
  const parts = p.split("/").filter(Boolean);
  return parts[parts.length - 1] ?? p;
>>>>>>> feature/staging-ui
}
`;

let releases: Release[] = [
  {
    id: "2",
    tag: "v0.2.0",
    name: "GitOut 0.2",
    body: "Changes since v0.1.0:\n\n- Hunk staging (abc1234)\n- Commit box with amend (def5678)",
    draft: false,
    prerelease: false,
    publishedAt: new Date((now - 86400 * 3) * 1000).toISOString(),
    createdAt: new Date((now - 86400 * 3) * 1000).toISOString(),
    author: "ada",
    url: "https://github.com/ada/gitout/releases/tag/v0.2.0",
    assets: [{ name: "gitout_0.2.0_amd64.deb", url: "https://example.com/gitout.deb", size: 9_830_400, downloads: 312 }],
  },
  {
    id: "1",
    tag: "v0.1.0",
    name: "",
    body: "",
    draft: false,
    prerelease: true,
    publishedAt: new Date((now - 86400 * 40) * 1000).toISOString(),
    createdAt: new Date((now - 86400 * 40) * 1000).toISOString(),
    author: "ada",
    url: "https://github.com/ada/gitout/releases/tag/v0.1.0",
    assets: [],
  },
];

function saveRelease(id: string | null, input: ReleaseInput): Release {
  const old = releases.find((r) => r.id === id);
  const r: Release = {
    id: old?.id ?? String(Date.now()),
    url: `https://github.com/ada/gitout/releases/tag/${input.tag}`,
    author: "ada",
    assets: old?.assets ?? [],
    createdAt: old?.createdAt ?? new Date().toISOString(),
    publishedAt: input.draft ? null : (old?.publishedAt ?? new Date().toISOString()),
    ...input,
  };
  releases = old ? releases.map((x) => (x === old ? r : x)) : [r, ...releases];
  return r;
}

// Workflow runs: the newest one runs live from page load, so the run view has progress to show.
const t0 = Date.now();
const iso = (ms: number) => new Date(ms).toISOString();
const JOBS: [string, string[]][] = [
  ["lint", ["Set up job", "Checkout", "Install", "Lint"]],
  ["build (ubuntu-latest)", ["Set up job", "Checkout", "Install Rust", "Cache", "Build", "Upload artifact"]],
  ["build (windows-latest)", ["Set up job", "Checkout", "Install Rust", "Cache", "Build", "Upload artifact"]],
  ["test", ["Set up job", "Checkout", "Install", "Unit tests", "Integration tests"]],
];
const STEP_MS = 3000;
function liveJobs(): CiJob[] {
  const elapsed = Date.now() - t0;
  return JOBS.map(([name, steps], ji) => {
    const start = t0 + ji * 1500;
    const at = (i: number) => start + i * STEP_MS;
    const doneSteps = Math.max(0, Math.min(steps.length, Math.floor((elapsed - ji * 1500) / STEP_MS)));
    const started = elapsed >= ji * 1500;
    const complete = doneSteps === steps.length;
    return {
      id: String(ji + 1),
      name,
      status: complete ? "completed" : started ? "running" : "queued",
      conclusion: complete ? "success" : null,
      startedAt: started ? iso(start) : null,
      completedAt: complete ? iso(at(steps.length)) : null,
      url: "https://github.com/ada/gitout/actions/runs/1",
      steps: steps.map((s, i) => ({
        number: i + 1,
        name: s,
        status: i < doneSteps ? "completed" : i === doneSteps && started ? "running" : "queued",
        conclusion: i < doneSteps ? "success" : null,
        startedAt: i <= doneSteps && started ? iso(at(i)) : null,
        completedAt: i < doneSteps ? iso(at(i + 1)) : null,
      })),
    } as CiJob;
  });
}
function runs(): CiRun[] {
  const live = liveJobs();
  const liveDone = live.every((j) => j.status === "completed");
  const base = { trigger: "push", attempt: 1, actor: "ada", url: "https://github.com/ada/gitout/actions/runs/1" };
  return [
    { ...base, id: "1", name: "CI", title: commits[0].subject, status: liveDone ? "completed" : "running", conclusion: liveDone ? "success" : null, branch: "main", headSha: commits[0].hash, number: 42, createdAt: iso(t0), startedAt: iso(t0), updatedAt: iso(Date.now()) },
    { ...base, id: "2", name: "Release", title: commits[1].subject, status: "completed", conclusion: "failure", branch: "main", headSha: commits[1].hash, number: 7, createdAt: iso(t0 - 3600_000), startedAt: iso(t0 - 3600_000), updatedAt: iso(t0 - 3300_000) },
    { ...base, id: "3", name: "CI", title: commits[1].subject, status: "completed", conclusion: "success", branch: "main", headSha: commits[1].hash, number: 41, createdAt: iso(t0 - 3600_000), startedAt: iso(t0 - 3600_000), updatedAt: iso(t0 - 3400_000) },
    { ...base, id: "4", name: "CI", title: "Staging panel", status: "completed", conclusion: "cancelled", branch: "feature/staging-ui", headSha: commits[3].hash, number: 40, trigger: "pull request", createdAt: iso(t0 - 86400_000), startedAt: iso(t0 - 86400_000), updatedAt: iso(t0 - 86000_000) },
  ];
}
const pause = (ms: number) => new Promise((r) => setTimeout(r, ms));

let settings: Settings = {
  recentRepos: [REPO, "/home/dev/projects/website", "/home/dev/work/api-server"],
  openRepos: [REPO, "/home/dev/projects/website", "/home/dev/work/api-server"],
  activeRepo: REPO,
  accounts: [{ id: "github:github.com:ada", provider: "github", host: "github.com", username: "ada", displayName: "Ada Lovelace", avatarUrl: null, gitUser: "x-access-token" }],
  theme: "system",
  pullMode: "merge",
  autoUpdate: false,
  oauth: {},
  lastSeenVersion: null,
};

const DIFF = `diff --git a/src/lib/graph.ts b/src/lib/graph.ts
index 1111111..2222222 100644
--- a/src/lib/graph.ts
+++ b/src/lib/graph.ts
@@ -10,7 +10,9 @@ export interface Segment {
   x1: number;
   y1: number;
-  x2: number;
+  x2: number; // destination lane
+  y2: number;
+  color: number;
 }

 export interface GraphRow {
@@ -40,6 +42,7 @@ export function layoutGraph(commits: Commit[]): GraphLayout {
   const lanes: (string | null)[] = [];
   const colors: number[] = [];
+  let nextColor = 0;
   let maxLanes = 1;
   const rows: GraphRow[] = [];
`;

export function installMock() {
  mockWindows("main");
  mockIPC(
    async (cmd, args) => {
      const a = (args ?? {}) as Record<string, unknown>;
      switch (cmd) {
        case "settings_get":
          return settings;
        case "settings_update":
          settings = { ...settings, ...(a.patch as object) };
          return settings;
        case "git_version":
          return "git version 2.47.0 (mock)";
        case "repo_open":
          return { path: a.path, name: String(a.path).split("/").pop() };
        case "status":
          // The other open repos are clean and behind, so "Fetch & pull all" has something to pull.
          return a.repo === REPO ? status : { ...status, files: [], ahead: 0, behind: String(a.repo).includes("website") ? 2 : 1 };
        case "log":
          return commits;
        case "refs":
          return refs;
        case "stashes":
          return [{ index: 0, hash: commits[5].hash, message: "WIP on main: experiment with lane widths" }];
        case "commit_info": {
          const c = byHash.get(String(a.hash)) ?? commits[0];
          return {
            ...c,
            committer: c.author,
            commitTime: c.time,
            message: `${c.subject}\n\nLonger description of the change goes here.`,
            files: [
              { path: "src/lib/graph.ts", origPath: null, status: "M" },
              { path: "src/lib/components/Graph.svelte", origPath: null, status: "A" },
            ],
          };
        }
        case "diff":
          return DIFF;
        case "release_host":
          return {
            provider: "github",
            host: "github.com",
            project: "ada/gitout",
            remote: "origin",
            accountId: "github:github.com:ada",
            webUrl: "https://github.com/ada/gitout/releases",
            supportsDrafts: true,
            supportsCi: true,
            problem: null,
          };
        case "ci_runs":
          return runs();
        case "ci_jobs":
          if (a.runId === "1") return liveJobs();
          return JOBS.slice(0, 2).map(([name, steps], i) => ({
            id: String(10 + i), name, status: "completed", conclusion: a.runId === "2" && i === 1 ? "failure" : "success",
            startedAt: iso(t0 - 3600_000), completedAt: iso(t0 - 3500_000), url: null,
            steps: steps.map((s, n) => ({ number: n + 1, name: s, status: "completed", conclusion: a.runId === "2" && i === 1 && n === 4 ? "failure" : a.runId === "2" && i === 1 && n > 4 ? "skipped" : "success", startedAt: iso(t0 - 3600_000 + n * 9000), completedAt: iso(t0 - 3600_000 + (n + 1) * 9000) })),
          }));
        case "fetch":
          await pause(700);
          return null;
        case "pull":
          await pause(500);
          if (String(a.repo).includes("api-server")) throw "error: Your local changes to the following files would be overwritten by merge:\n\tsrc/server.ts";
          return null;
        case "releases_list":
          return releases;
        case "release_create":
          return saveRelease(null, a.input as ReleaseInput);
        case "release_update":
          return saveRelease(String(a.id), a.input as ReleaseInput);
        case "release_delete":
          releases = releases.filter((r) => r.id !== a.id);
          return null;
        case "release_changelog":
          return { previous: "v0.1.0", lines: commits.slice(0, 5).map((c) => `${c.subject} (${c.hash.slice(0, 7)})`) };
        case "conflict_file": {
          const binary = a.file === "docs/logo.png";
          return {
            working: binary ? null : CONFLICTED,
            binary,
            hasBase: true,
            ours: { title: "Current", detail: "main", exists: true, text: binary ? null : "" },
            theirs: { title: "Incoming", detail: "feature/staging-ui", exists: true, text: binary ? null : "" },
          };
        }
        case "conflict_save":
        case "conflict_take":
          if (a.stage !== false) status.files = status.files.filter((f) => f.path !== a.file);
          if (!status.files.some((f) => f.conflicted)) status.state = null;
          return null;
        case "auth_list_repos":
          return [
            { fullName: "ada/analytical-engine", description: "Notes on the engine", private: false, httpsUrl: "https://github.com/ada/analytical-engine.git", sshUrl: "git@github.com:ada/analytical-engine.git" },
            { fullName: "ada/gitout", description: null, private: true, httpsUrl: "https://github.com/ada/gitout.git", sshUrl: "git@github.com:ada/gitout.git" },
          ];
        case "plugin:app|version":
          return "0.1.0";
        case "plugin:path|resolve_directory":
          return "/home/dev";
        case "plugin:path|join":
          return (a.paths as string[]).join("/");
        default:
          console.debug("[mock] unhandled", cmd, a);
          return null;
      }
    },
    { shouldMockEvents: true },
  );
}
