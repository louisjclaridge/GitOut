//! Thin wrapper around the system `git` binary plus parsers for its
//! machine-readable output formats. We shell out (rather than embedding
//! libgit2) so users get exactly their own git: SSH keys, credential
//! helpers, hooks, config and LFS all behave as they do in a terminal.

use crate::error::{Error, Result};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, Stdio};

/// Fields in log/ref formats are separated by this byte; records by NUL.
const FS: char = '\x1f';

#[derive(Default)]
pub struct Opts<'a> {
    pub stdin: Option<&'a [u8]>,
    pub env: Vec<(String, String)>,
    /// Exit codes other than 0 that should be treated as success.
    pub ok_codes: &'a [i32],
}

fn base_command(repo: Option<&Path>) -> Command {
    let mut c = Command::new("git");
    if let Some(r) = repo {
        c.current_dir(r);
    }
    c.args(["-c", "core.quotepath=false", "-c", "color.ui=false", "-c", "log.showSignature=false"])
        // Never block waiting for a terminal prompt that no one can see.
        .env("GIT_TERMINAL_PROMPT", "0")
        // Paths we pass are always literal file names, never pathspec globs.
        .env("GIT_LITERAL_PATHSPECS", "1")
        .env("GIT_OPTIONAL_LOCKS", "0");
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

fn spawn_err(e: std::io::Error) -> Error {
    if e.kind() == std::io::ErrorKind::NotFound {
        Error::GitMissing
    } else {
        Error::Io(e)
    }
}

fn failure(code: Option<i32>, stdout: &[u8], stderr: &[u8]) -> Error {
    let err = String::from_utf8_lossy(stderr).trim().to_string();
    let msg = if err.is_empty() { String::from_utf8_lossy(stdout).trim().to_string() } else { err };
    if msg.is_empty() {
        Error::Git(format!("git exited with code {}", code.unwrap_or(-1)))
    } else {
        Error::Git(msg)
    }
}

pub fn run_with<S: AsRef<std::ffi::OsStr>>(repo: Option<&Path>, args: &[S], opts: Opts) -> Result<String> {
    let mut cmd = base_command(repo);
    cmd.args(args).envs(opts.env).stdout(Stdio::piped()).stderr(Stdio::piped());
    cmd.stdin(if opts.stdin.is_some() { Stdio::piped() } else { Stdio::null() });
    let mut child = cmd.spawn().map_err(spawn_err)?;
    if let Some(input) = opts.stdin {
        // Write on a separate thread so a large patch can't deadlock against a full stdout pipe.
        let mut pipe = child.stdin.take().expect("stdin piped");
        let input = input.to_vec();
        std::thread::spawn(move || {
            let _ = pipe.write_all(&input);
        });
    }
    let out = child.wait_with_output()?;
    let code = out.status.code();
    if out.status.success() || code.is_some_and(|c| opts.ok_codes.contains(&c)) {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(failure(code, &out.stdout, &out.stderr))
    }
}

pub fn run<S: AsRef<std::ffi::OsStr>>(repo: &Path, args: &[S]) -> Result<String> {
    run_with(Some(repo), args, Opts::default())
}

/// Run a long network operation, reporting each progress line from stderr
/// (git separates progress updates with `\r`) to `on_progress`.
pub fn run_streaming<S: AsRef<std::ffi::OsStr>>(
    repo: Option<&Path>,
    args: &[S],
    env: Vec<(String, String)>,
    mut on_progress: impl FnMut(&str),
) -> Result<String> {
    let mut cmd = base_command(repo);
    cmd.args(args).envs(env).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(spawn_err)?;

    let mut stdout = child.stdout.take().expect("stdout piped");
    let out_thread = std::thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout.read_to_end(&mut buf);
        buf
    });

    let mut stderr = child.stderr.take().expect("stderr piped");
    let mut all_err = Vec::new();
    let mut line = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        let n = stderr.read(&mut chunk)?;
        if n == 0 {
            break;
        }
        for &b in &chunk[..n] {
            all_err.push(b);
            if b == b'\r' || b == b'\n' {
                if !line.is_empty() {
                    on_progress(String::from_utf8_lossy(&line).trim());
                    line.clear();
                }
            } else {
                line.push(b);
            }
        }
    }
    let status = child.wait()?;
    let stdout = out_thread.join().unwrap_or_default();
    if status.success() {
        let mut s = String::from_utf8_lossy(&stdout).into_owned();
        s.push_str(&String::from_utf8_lossy(&all_err));
        Ok(s)
    } else {
        // Progress noise ("Counting objects: 50%") isn't useful in an error; keep the rest.
        let useful: Vec<&str> = std::str::from_utf8(&all_err)
            .unwrap_or("")
            .split(['\r', '\n'])
            .filter(|l| {
                let l = l.trim();
                !l.is_empty() && !l.contains('%') && !l.starts_with("remote: Enumerating")
            })
            .collect();
        Err(failure(status.code(), &stdout, useful.join("\n").as_bytes()))
    }
}

// ---------------------------------------------------------------- status

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChange {
    pub path: String,
    pub orig_path: Option<String>,
    /// Single-letter index (staged) status: M A D R C T U or '.'
    pub index: char,
    /// Single-letter worktree status, or '?' for untracked.
    pub worktree: char,
    pub conflicted: bool,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub branch: Option<String>,
    pub head: Option<String>,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub files: Vec<FileChange>,
    /// "merging" | "rebasing" | "cherry-picking" | "reverting" | null
    pub state: Option<String>,
}

/// Parse `git status --porcelain=v2 --branch -z --untracked-files=all`.
pub fn parse_status(out: &str) -> Status {
    let mut st = Status::default();
    let mut recs = out.split('\0');
    while let Some(rec) = recs.next() {
        if rec.is_empty() {
            continue;
        }
        if let Some(h) = rec.strip_prefix("# ") {
            let (key, val) = h.split_once(' ').unwrap_or((h, ""));
            match key {
                "branch.oid" if val != "(initial)" => st.head = Some(val.into()),
                "branch.head" if val != "(detached)" => st.branch = Some(val.into()),
                "branch.upstream" => st.upstream = Some(val.into()),
                "branch.ab" => {
                    for part in val.split(' ') {
                        if let Some(n) = part.strip_prefix('+') {
                            st.ahead = n.parse().unwrap_or(0);
                        } else if let Some(n) = part.strip_prefix('-') {
                            st.behind = n.parse().unwrap_or(0);
                        }
                    }
                }
                _ => {}
            }
            continue;
        }
        let kind = rec.as_bytes()[0];
        match kind {
            b'1' | b'2' => {
                // 1 XY sub mH mI mW hH hI path
                // 2 XY sub mH mI mW hH hI Xscore path   (followed by a separate origPath record)
                let n = if kind == b'1' { 9 } else { 10 };
                let f: Vec<&str> = rec.splitn(n, ' ').collect();
                if f.len() < n {
                    continue;
                }
                let xy: Vec<char> = f[1].chars().collect();
                let orig = if kind == b'2' { recs.next().map(String::from) } else { None };
                st.files.push(FileChange {
                    path: f[n - 1].into(),
                    orig_path: orig,
                    index: xy[0],
                    worktree: xy[1],
                    conflicted: false,
                });
            }
            b'u' => {
                let f: Vec<&str> = rec.splitn(11, ' ').collect();
                if let Some(p) = f.get(10) {
                    st.files.push(FileChange {
                        path: (*p).into(),
                        orig_path: None,
                        index: 'U',
                        worktree: 'U',
                        conflicted: true,
                    });
                }
            }
            b'?' => st.files.push(FileChange {
                path: rec[2..].into(),
                orig_path: None,
                index: '.',
                worktree: '?',
                conflicted: false,
            }),
            _ => {}
        }
    }
    st
}

pub fn repo_state(git_dir: &Path) -> Option<String> {
    let has = |p: &str| git_dir.join(p).exists();
    if has("rebase-merge") || has("rebase-apply") {
        Some("rebasing".into())
    } else if has("MERGE_HEAD") {
        Some("merging".into())
    } else if has("CHERRY_PICK_HEAD") {
        Some("cherry-picking".into())
    } else if has("REVERT_HEAD") {
        Some("reverting".into())
    } else {
        None
    }
}

// ---------------------------------------------------------------- log

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Commit {
    pub hash: String,
    pub parents: Vec<String>,
    pub author: String,
    pub email: String,
    pub time: i64,
    pub refs: Vec<String>,
    pub subject: String,
}

pub const LOG_FORMAT: &str = "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%D%x1f%s";

/// Parse `git log -z --decorate=full` output in `LOG_FORMAT`.
pub fn parse_log(out: &str) -> Vec<Commit> {
    out.split('\0')
        .filter_map(|rec| {
            let rec = rec.trim_start_matches('\n');
            let f: Vec<&str> = rec.splitn(7, FS).collect();
            if f.len() < 7 {
                return None;
            }
            Some(Commit {
                hash: f[0].into(),
                parents: f[1].split_whitespace().map(String::from).collect(),
                author: f[2].into(),
                email: f[3].into(),
                time: f[4].parse().unwrap_or(0),
                refs: parse_decorations(f[5]),
                subject: f[6].into(),
            })
        })
        .collect()
}

/// "HEAD -> refs/heads/main, refs/remotes/origin/main, tag: refs/tags/v1"
/// becomes ["HEAD", "refs/heads/main", "refs/remotes/origin/main", "refs/tags/v1"].
fn parse_decorations(d: &str) -> Vec<String> {
    let mut refs = vec![];
    for part in d.split(", ").filter(|p| !p.is_empty()) {
        if let Some((head, target)) = part.split_once(" -> ") {
            refs.push(head.to_string());
            refs.push(target.to_string());
        } else if let Some(t) = part.strip_prefix("tag: ") {
            refs.push(t.to_string());
        } else {
            refs.push(part.to_string());
        }
    }
    refs
}

// ---------------------------------------------------------------- refs

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Ref {
    /// Full ref name, e.g. refs/heads/main
    pub name: String,
    /// Display name, e.g. main or origin/main
    pub short: String,
    /// "local" | "remote" | "tag"
    pub kind: String,
    pub target: String,
    pub upstream: Option<String>,
    pub ahead: u32,
    pub behind: u32,
    pub current: bool,
}

pub const REF_FORMAT: &str =
    "--format=%(refname)%1f%(objectname)%1f%(*objectname)%1f%(upstream:short)%1f%(upstream:track,nobracket)%1f%(HEAD)";

pub fn parse_refs(out: &str) -> Vec<Ref> {
    out.lines()
        .filter_map(|line| {
            let f: Vec<&str> = line.split(FS).collect();
            if f.len() < 6 {
                return None;
            }
            let name = f[0];
            let (kind, short) = if let Some(s) = name.strip_prefix("refs/heads/") {
                ("local", s)
            } else if let Some(s) = name.strip_prefix("refs/remotes/") {
                if s.ends_with("/HEAD") {
                    return None;
                }
                ("remote", s)
            } else if let Some(s) = name.strip_prefix("refs/tags/") {
                ("tag", s)
            } else {
                return None;
            };
            let (mut ahead, mut behind) = (0, 0);
            for part in f[4].split(", ") {
                if let Some(n) = part.strip_prefix("ahead ") {
                    ahead = n.parse().unwrap_or(0);
                } else if let Some(n) = part.strip_prefix("behind ") {
                    behind = n.parse().unwrap_or(0);
                }
            }
            // Annotated tags point at a tag object; %(*objectname) is the commit it peels to.
            let target = if f[2].is_empty() { f[1] } else { f[2] };
            Some(Ref {
                name: name.into(),
                short: short.into(),
                kind: kind.into(),
                target: target.into(),
                upstream: (!f[3].is_empty()).then(|| f[3].into()),
                ahead,
                behind,
                current: f[5] == "*",
            })
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stash {
    pub index: u32,
    pub hash: String,
    pub message: String,
}

pub fn parse_stashes(out: &str) -> Vec<Stash> {
    out.lines()
        .enumerate()
        .filter_map(|(i, line)| {
            let (hash, msg) = line.split_once(FS)?;
            Some(Stash { index: i as u32, hash: hash.into(), message: msg.into() })
        })
        .collect()
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChangedFile {
    pub path: String,
    pub orig_path: Option<String>,
    pub status: char,
}

/// Parse `--name-status -z` output: STATUS\0path\0 or Rnn\0old\0new\0.
pub fn parse_name_status(out: &str) -> Vec<ChangedFile> {
    let mut files = vec![];
    let mut it = out.split('\0').filter(|s| !s.is_empty());
    while let Some(code) = it.next() {
        let status = code.chars().next().unwrap_or('M');
        if status == 'R' || status == 'C' {
            let old = it.next().unwrap_or_default();
            let new = it.next().unwrap_or_default();
            files.push(ChangedFile { path: new.into(), orig_path: Some(old.into()), status });
        } else if let Some(p) = it.next() {
            files.push(ChangedFile { path: p.into(), orig_path: None, status });
        }
    }
    files
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_v2() {
        let out = "# branch.oid abc\0# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0\
1 .M N... 100644 100644 100644 aaa bbb src/a b.rs\0\
2 R. N... 100644 100644 100644 aaa bbb R100 new.rs\0old.rs\0\
u UU N... 100644 100644 100644 100644 a b c conflict.txt\0\
? untracked.txt\0";
        let s = parse_status(out);
        assert_eq!(s.branch.as_deref(), Some("main"));
        assert_eq!((s.ahead, s.behind), (2, 1));
        assert_eq!(s.files.len(), 4);
        assert_eq!(s.files[0].path, "src/a b.rs");
        assert_eq!(s.files[1].orig_path.as_deref(), Some("old.rs"));
        assert!(s.files[2].conflicted);
        assert_eq!(s.files[3].worktree, '?');
    }

    #[test]
    fn log_and_decorations() {
        let out = "h1\x1fp1 p2\x1fAnn\x1fa@x\x1f100\x1fHEAD -> refs/heads/main, tag: refs/tags/v1\x1fMerge it\0\nh2\x1f\x1fBob\x1fb@x\x1f50\x1f\x1fInit\0";
        let c = parse_log(out);
        assert_eq!(c.len(), 2);
        assert_eq!(c[0].parents, vec!["p1", "p2"]);
        assert_eq!(c[0].refs, vec!["HEAD", "refs/heads/main", "refs/tags/v1"]);
        assert!(c[1].parents.is_empty());
    }

    #[test]
    fn name_status() {
        let f = parse_name_status("M\0a.txt\0R090\0old\0new\0");
        assert_eq!(f.len(), 2);
        assert_eq!(f[1].path, "new");
    }
}
