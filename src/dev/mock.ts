// Development-only fake backend so the UI can be worked on in a plain
// browser (`npm run dev`, then open http://localhost:1420) without Rust.
// Loaded from main.ts only when not running inside Tauri.
import { mockIPC, mockWindows } from "@tauri-apps/api/mocks";
import type { Commit, Ref, Settings, Status } from "../lib/api";

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

let settings: Settings = {
  recentRepos: [REPO, "/home/dev/projects/website", "/home/dev/work/api-server"],
  openRepos: [REPO],
  activeRepo: REPO,
  accounts: [{ id: "github:github.com:ada", provider: "github", host: "github.com", username: "ada", displayName: "Ada Lovelace", avatarUrl: null, gitUser: "x-access-token" }],
  theme: "system",
  pullMode: "merge",
  autoUpdate: false,
  oauth: {},
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
    (cmd, args) => {
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
          return status;
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
