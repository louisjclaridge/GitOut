//! Tauri commands exposed to the frontend. Every git invocation runs on a
//! blocking thread so the UI never stalls on a slow repository.

use crate::auth::{self, DeviceStart, RemoteRepo, TokenStore};
use crate::error::{Error, Result};
use crate::git::{self, ChangedFile, Commit, Opts, Ref, Stash, Status};
use crate::store::{Account, OAuthOverrides, Settings, Store};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Emitter, State};

pub struct AppState {
    pub store: Store,
    pub tokens: TokenStore,
    /// Bumped to cancel any in-flight device-flow sign-in.
    pub auth_generation: AtomicU64,
}

async fn blocking<T: Send + 'static>(f: impl FnOnce() -> Result<T> + Send + 'static) -> Result<T> {
    tauri::async_runtime::spawn_blocking(f).await.map_err(|e| Error::Other(e.to_string()))?
}

fn path(repo: &str) -> PathBuf {
    PathBuf::from(repo)
}

#[derive(Clone, Serialize)]
struct Progress<'a> {
    repo: &'a str,
    line: &'a str,
}

/// Run a network operation with account credentials injected and progress
/// lines forwarded to the frontend as `git-progress` events.
async fn network_op(app: AppHandle, state: &AppState, repo: Option<String>, args: Vec<String>) -> Result<String> {
    let accounts = state.store.get().accounts;
    let env = auth::git_auth_env(&state.tokens, &accounts).await;
    blocking(move || {
        let label = repo.clone().unwrap_or_default();
        git::run_streaming(repo.as_deref().map(Path::new), &args, env, |line| {
            let _ = app.emit("git-progress", Progress { repo: &label, line });
        })
    })
    .await
}

// ---------------------------------------------------------------- settings

#[tauri::command]
pub fn settings_get(state: State<AppState>) -> Settings {
    state.store.get()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsPatch {
    theme: Option<String>,
    pull_mode: Option<String>,
    auto_update: Option<bool>,
    oauth: Option<OAuthOverrides>,
    open_repos: Option<Vec<String>>,
    /// Empty string clears it.
    active_repo: Option<String>,
    recent_repos: Option<Vec<String>>,
}

#[tauri::command]
pub fn settings_update(state: State<AppState>, patch: SettingsPatch) -> Result<Settings> {
    state.store.update(|s| {
        if let Some(v) = patch.theme { s.theme = v }
        if let Some(v) = patch.pull_mode { s.pull_mode = v }
        if let Some(v) = patch.auto_update { s.auto_update = v }
        if let Some(v) = patch.oauth { s.oauth = v }
        if let Some(v) = patch.open_repos { s.open_repos = v }
        if let Some(v) = patch.active_repo { s.active_repo = (!v.is_empty()).then_some(v) }
        if let Some(v) = patch.recent_repos { s.recent_repos = v }
    })
}

// ---------------------------------------------------------------- repositories

#[derive(Serialize)]
pub struct RepoInfo {
    path: String,
    name: String,
}

fn repo_info(dir: &Path) -> Result<RepoInfo> {
    let top = git::run(dir, &["rev-parse", "--show-toplevel"])
        .map_err(|_| Error::Git(format!("{} is not a git repository", dir.display())))?;
    let top = PathBuf::from(top.trim());
    // Normalise separators on Windows (git prints forward slashes).
    let top = top.canonicalize().map(dunce).unwrap_or(top);
    let name = top.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    Ok(RepoInfo { path: top.to_string_lossy().into_owned(), name })
}

/// Strip the `\\?\` verbatim prefix `canonicalize` adds on Windows.
fn dunce(p: PathBuf) -> PathBuf {
    let s = p.to_string_lossy();
    match s.strip_prefix(r"\\?\") {
        Some(rest) if !rest.starts_with("UNC") => PathBuf::from(rest),
        _ => p,
    }
}

#[tauri::command]
pub async fn git_version() -> Result<String> {
    blocking(|| git::run_with(None, &["--version"], Opts::default()).map(|s| s.trim().to_string())).await
}

#[tauri::command]
pub async fn repo_open(state: State<'_, AppState>, path: String) -> Result<RepoInfo> {
    let info = blocking(move || repo_info(Path::new(&path))).await?;
    state.store.touch_recent(&info.path)?;
    Ok(info)
}

#[tauri::command]
pub async fn repo_init(state: State<'_, AppState>, path: String) -> Result<RepoInfo> {
    let info = blocking(move || {
        std::fs::create_dir_all(&path)?;
        git::run(Path::new(&path), &["init"])?;
        repo_info(Path::new(&path))
    })
    .await?;
    state.store.touch_recent(&info.path)?;
    Ok(info)
}

#[tauri::command]
pub async fn repo_clone(
    app: AppHandle,
    state: State<'_, AppState>,
    url: String,
    parent: String,
    name: String,
) -> Result<RepoInfo> {
    let dest = Path::new(&parent).join(name.trim());
    if dest.exists() && dest.read_dir().map(|mut d| d.next().is_some()).unwrap_or(true) {
        return Err(Error::Other(format!("{} already exists and is not empty", dest.display())));
    }
    let dest_s = dest.to_string_lossy().into_owned();
    network_op(app, &state, None, vec!["clone".into(), "--progress".into(), "--".into(), url, dest_s]).await?;
    let info = blocking(move || repo_info(&dest)).await?;
    state.store.touch_recent(&info.path)?;
    Ok(info)
}

#[derive(Serialize)]
pub struct Remote {
    name: String,
    url: String,
}

#[tauri::command]
pub async fn remotes(repo: String) -> Result<Vec<Remote>> {
    blocking(move || {
        let out = git::run(&path(&repo), &["remote", "-v"])?;
        let mut v: Vec<Remote> = vec![];
        for line in out.lines() {
            let mut it = line.split_whitespace();
            if let (Some(name), Some(url)) = (it.next(), it.next()) {
                if !v.iter().any(|r| r.name == name) {
                    v.push(Remote { name: name.into(), url: url.into() });
                }
            }
        }
        Ok(v)
    })
    .await
}

// ---------------------------------------------------------------- reading

#[tauri::command]
pub async fn status(repo: String) -> Result<Status> {
    blocking(move || {
        let p = path(&repo);
        let out = git::run(&p, &["status", "--porcelain=v2", "--branch", "-z", "--untracked-files=all"])?;
        let mut st = git::parse_status(&out);
        let git_dir = git::run(&p, &["rev-parse", "--absolute-git-dir"])?;
        st.state = git::repo_state(Path::new(git_dir.trim()));
        Ok(st)
    })
    .await
}

#[tauri::command]
pub async fn log(repo: String, limit: u32) -> Result<Vec<Commit>> {
    blocking(move || {
        let p = path(&repo);
        let has_head = git::run(&p, &["rev-parse", "--verify", "-q", "HEAD"]).is_ok();
        let n = format!("-n{limit}");
        let mut args = vec!["log", "-z", "--date-order", "--decorate=full", git::LOG_FORMAT, n.as_str(), "--branches", "--remotes", "--tags"];
        if has_head {
            args.push("HEAD");
        }
        match git::run(&p, &args) {
            Ok(out) => Ok(git::parse_log(&out)),
            // A brand-new repository has nothing to show yet.
            Err(_) if !has_head => Ok(vec![]),
            Err(e) => Err(e),
        }
    })
    .await
}

#[tauri::command]
pub async fn refs(repo: String) -> Result<Vec<Ref>> {
    blocking(move || {
        let out = git::run(
            &path(&repo),
            &["for-each-ref", "--sort=-committerdate", git::REF_FORMAT, "refs/heads", "refs/remotes", "refs/tags"],
        )?;
        Ok(git::parse_refs(&out))
    })
    .await
}

#[tauri::command]
pub async fn stashes(repo: String) -> Result<Vec<Stash>> {
    blocking(move || {
        let out = git::run(&path(&repo), &["stash", "list", "--format=%H%x1f%s"])?;
        Ok(git::parse_stashes(&out))
    })
    .await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommitInfo {
    hash: String,
    parents: Vec<String>,
    author: String,
    email: String,
    time: i64,
    committer: String,
    commit_time: i64,
    message: String,
    files: Vec<ChangedFile>,
}

#[tauri::command]
pub async fn commit_info(repo: String, hash: String) -> Result<CommitInfo> {
    blocking(move || {
        let p = path(&repo);
        let out = git::run(&p, &["show", "-s", "--format=%H%x1f%P%x1f%an%x1f%ae%x1f%at%x1f%cn%x1f%ct%x1f%B", &hash, "--"])?;
        let f: Vec<&str> = out.splitn(8, '\x1f').collect();
        if f.len() < 8 {
            return Err(Error::Git(format!("could not read commit {hash}")));
        }
        let parents: Vec<String> = f[1].split_whitespace().map(String::from).collect();
        let files_out = match parents.first() {
            Some(parent) => git::run(&p, &["diff", "--no-ext-diff", "--name-status", "-z", "-M", parent, &hash, "--"])?,
            None => git::run(&p, &["diff-tree", "--root", "-r", "--no-commit-id", "--name-status", "-z", "-M", &hash])?,
        };
        Ok(CommitInfo {
            hash: f[0].into(),
            parents,
            author: f[2].into(),
            email: f[3].into(),
            time: f[4].parse().unwrap_or(0),
            committer: f[5].into(),
            commit_time: f[6].parse().unwrap_or(0),
            message: f[7].trim_end().into(),
            files: git::parse_name_status(&files_out),
        })
    })
    .await
}

/// Unified diff for one file. `mode` is "unstaged", "staged", "untracked" or
/// "commit" (with `hash`, diffed against its first parent).
#[tauri::command]
pub async fn diff(
    repo: String,
    file: String,
    orig: Option<String>,
    mode: String,
    hash: Option<String>,
) -> Result<String> {
    blocking(move || {
        let p = path(&repo);
        let mut args: Vec<String> = vec!["diff".into(), "--no-ext-diff".into(), "-M".into()];
        let mut ok_codes: &[i32] = &[];
        match mode.as_str() {
            "unstaged" => {}
            "staged" => args.push("--cached".into()),
            "untracked" => {
                args.extend(["--no-index".into(), "--".into(), "/dev/null".into(), file.clone()]);
                ok_codes = &[1];
            }
            "commit" => {
                let hash = hash.ok_or_else(|| Error::Other("commit diff needs a hash".into()))?;
                let parent = git::run(&p, &["rev-parse", "-q", "--verify", &format!("{hash}^1")]).ok();
                match parent {
                    Some(par) => args.extend([par.trim().to_string(), hash]),
                    // Root commit: diff against the empty tree.
                    None => {
                        let empty = git::run_with(
                            Some(&p),
                            &["hash-object", "-t", "tree", "--stdin"],
                            Opts { stdin: Some(b"".as_slice()), ..Default::default() },
                        )?;
                        args.extend([empty.trim().to_string(), hash]);
                    }
                }
            }
            m => return Err(Error::Other(format!("unknown diff mode {m}"))),
        }
        if mode != "untracked" {
            args.push("--".into());
            if let Some(o) = orig {
                args.push(o);
            }
            args.push(file);
        }
        git::run_with(Some(&p), &args, Opts { ok_codes, ..Default::default() })
    })
    .await
}

// ---------------------------------------------------------------- working tree

fn has_head(p: &Path) -> bool {
    git::run(p, &["rev-parse", "--verify", "-q", "HEAD"]).is_ok()
}

#[tauri::command]
pub async fn stage(repo: String, paths: Vec<String>) -> Result<()> {
    blocking(move || {
        let mut args = vec!["add".to_string(), "-A".into(), "--".into()];
        args.extend(paths);
        git::run(&path(&repo), &args).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn unstage(repo: String, paths: Vec<String>) -> Result<()> {
    blocking(move || {
        let p = path(&repo);
        let mut args: Vec<String> = if has_head(&p) {
            vec!["restore".into(), "--staged".into(), "--".into()]
        } else {
            // No commits yet: nothing to restore from, so just drop from the index.
            vec!["rm".into(), "--cached".into(), "-r".into(), "-q".into(), "--".into()]
        };
        args.extend(paths);
        git::run(&p, &args).map(|_| ())
    })
    .await
}

/// Throw away unstaged changes. Untracked files are deleted.
#[tauri::command]
pub async fn discard(repo: String, paths: Vec<String>, untracked: Vec<String>) -> Result<()> {
    blocking(move || {
        let p = path(&repo);
        for f in &untracked {
            let full = p.join(f);
            if full.is_dir() {
                std::fs::remove_dir_all(full)?;
            } else {
                std::fs::remove_file(full)?;
            }
        }
        if !paths.is_empty() {
            let mut args = vec!["restore".to_string(), "--worktree".into(), "--".into()];
            args.extend(paths);
            git::run(&p, &args)?;
        }
        Ok(())
    })
    .await
}

/// Apply a patch (a single hunk) to the index and/or working tree.
/// Used for hunk-level stage, unstage and discard.
#[tauri::command]
pub async fn apply_patch(repo: String, patch: String, cached: bool, reverse: bool) -> Result<()> {
    blocking(move || {
        let mut args = vec!["apply", "--whitespace=nowarn", "--recount"];
        if cached {
            args.push("--cached");
        }
        if reverse {
            args.push("--reverse");
        }
        args.push("-");
        git::run_with(Some(&path(&repo)), &args, Opts { stdin: Some(patch.as_bytes()), ..Default::default() })
            .map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn commit(repo: String, message: String, amend: bool) -> Result<()> {
    blocking(move || {
        let mut args = vec!["commit", "--cleanup=strip", "-F", "-"];
        if amend {
            args.push("--amend");
        }
        git::run_with(Some(&path(&repo)), &args, Opts { stdin: Some(message.as_bytes()), ..Default::default() })
            .map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn head_message(repo: String) -> Result<String> {
    blocking(move || git::run(&path(&repo), &["log", "-1", "--format=%B"]).map(|s| s.trim_end().to_string())).await
}

// ---------------------------------------------------------------- branches & history

/// `kind`: "local" (branch name), "remote" (e.g. origin/feature) or "commit" (hash, detached).
#[tauri::command]
pub async fn checkout(repo: String, target: String, kind: String) -> Result<()> {
    blocking(move || {
        let p = path(&repo);
        let r = match kind.as_str() {
            "local" => git::run(&p, &["switch", &target]),
            "remote" => {
                let local = target.split_once('/').map(|(_, b)| b).unwrap_or(&target).to_string();
                let exists = git::run(&p, &["rev-parse", "--verify", "-q", &format!("refs/heads/{local}")]).is_ok();
                if exists {
                    git::run(&p, &["switch", &local])
                } else {
                    git::run(&p, &["switch", "-c", &local, "--track", &target])
                }
            }
            _ => git::run(&p, &["switch", "--detach", &target]),
        };
        r.map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn branch_create(repo: String, name: String, start: Option<String>, checkout: bool) -> Result<()> {
    blocking(move || {
        let p = path(&repo);
        let mut args = if checkout { vec!["switch", "-c", &name] } else { vec!["branch", &name] };
        if let Some(s) = start.as_deref() {
            args.push(s);
        }
        git::run(&p, &args).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn branch_rename(repo: String, old: String, new: String) -> Result<()> {
    blocking(move || git::run(&path(&repo), &["branch", "-m", &old, &new]).map(|_| ())).await
}

#[tauri::command]
pub async fn branch_delete(repo: String, name: String, force: bool) -> Result<()> {
    blocking(move || git::run(&path(&repo), &["branch", if force { "-D" } else { "-d" }, &name]).map(|_| ())).await
}

/// Delete a branch on its remote, e.g. "origin/feature".
#[tauri::command]
pub async fn remote_branch_delete(app: AppHandle, state: State<'_, AppState>, repo: String, remote_ref: String) -> Result<()> {
    let (remote, branch) =
        remote_ref.split_once('/').ok_or_else(|| Error::Other(format!("{remote_ref} is not a remote branch")))?;
    let args = vec!["push".into(), "--progress".into(), remote.into(), "--delete".into(), branch.into()];
    network_op(app, &state, Some(repo), args).await.map(|_| ())
}

fn run_editorless(p: &Path, args: &[&str]) -> Result<String> {
    // Accept git's default messages for merge/revert/rebase-continue without opening an editor.
    git::run_with(Some(p), args, Opts { env: vec![("GIT_EDITOR".into(), "true".into())], ..Default::default() })
}

#[tauri::command]
pub async fn merge(repo: String, target: String) -> Result<()> {
    blocking(move || run_editorless(&path(&repo), &["merge", "--no-edit", &target]).map(|_| ())).await
}

#[tauri::command]
pub async fn rebase(repo: String, onto: String) -> Result<()> {
    blocking(move || run_editorless(&path(&repo), &["rebase", &onto]).map(|_| ())).await
}

#[tauri::command]
pub async fn cherry_pick(repo: String, hash: String) -> Result<()> {
    blocking(move || run_editorless(&path(&repo), &["cherry-pick", &hash]).map(|_| ())).await
}

#[tauri::command]
pub async fn revert(repo: String, hash: String) -> Result<()> {
    blocking(move || run_editorless(&path(&repo), &["revert", "--no-edit", &hash]).map(|_| ())).await
}

/// `mode`: "soft" | "mixed" | "hard"
#[tauri::command]
pub async fn reset(repo: String, hash: String, mode: String) -> Result<()> {
    if !matches!(mode.as_str(), "soft" | "mixed" | "hard") {
        return Err(Error::Other(format!("invalid reset mode {mode}")));
    }
    blocking(move || git::run(&path(&repo), &["reset", &format!("--{mode}"), &hash]).map(|_| ())).await
}

fn op_for_state(st: &Option<String>) -> Result<&'static str> {
    match st.as_deref() {
        Some("merging") => Ok("merge"),
        Some("rebasing") => Ok("rebase"),
        Some("cherry-picking") => Ok("cherry-pick"),
        Some("reverting") => Ok("revert"),
        _ => Err(Error::Other("No merge, rebase, cherry-pick or revert is in progress".into())),
    }
}

fn current_state(p: &Path) -> Result<Option<String>> {
    let dir = git::run(p, &["rev-parse", "--absolute-git-dir"])?;
    Ok(git::repo_state(Path::new(dir.trim())))
}

#[tauri::command]
pub async fn operation_abort(repo: String) -> Result<()> {
    blocking(move || {
        let p = path(&repo);
        let op = op_for_state(&current_state(&p)?)?;
        git::run(&p, &[op, "--abort"]).map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn operation_continue(repo: String) -> Result<()> {
    blocking(move || {
        let p = path(&repo);
        let r = match op_for_state(&current_state(&p)?)? {
            "merge" => run_editorless(&p, &["commit", "--no-edit"]),
            op => run_editorless(&p, &[op, "--continue"]),
        };
        r.map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn tag_create(repo: String, name: String, target: String, message: Option<String>) -> Result<()> {
    blocking(move || {
        let p = path(&repo);
        let r = match message.filter(|m| !m.trim().is_empty()) {
            Some(m) => git::run(&p, &["tag", "-a", &name, "-m", &m, &target]),
            None => git::run(&p, &["tag", &name, &target]),
        };
        r.map(|_| ())
    })
    .await
}

#[tauri::command]
pub async fn tag_delete(repo: String, name: String) -> Result<()> {
    blocking(move || git::run(&path(&repo), &["tag", "-d", &name]).map(|_| ())).await
}

// ---------------------------------------------------------------- stash

#[tauri::command]
pub async fn stash_save(repo: String, message: Option<String>, include_untracked: bool) -> Result<()> {
    blocking(move || {
        let mut args = vec!["stash".to_string(), "push".into()];
        if include_untracked {
            args.push("--include-untracked".into());
        }
        if let Some(m) = message.filter(|m| !m.trim().is_empty()) {
            args.extend(["-m".into(), m]);
        }
        git::run(&path(&repo), &args).map(|_| ())
    })
    .await
}

/// `action`: "apply" | "pop" | "drop"
#[tauri::command]
pub async fn stash_action(repo: String, index: u32, action: String) -> Result<()> {
    if !matches!(action.as_str(), "apply" | "pop" | "drop") {
        return Err(Error::Other(format!("invalid stash action {action}")));
    }
    blocking(move || git::run(&path(&repo), &["stash", &action, &format!("stash@{{{index}}}")]).map(|_| ())).await
}

// ---------------------------------------------------------------- sync

#[tauri::command]
pub async fn fetch(app: AppHandle, state: State<'_, AppState>, repo: String) -> Result<()> {
    let args = ["fetch", "--all", "--prune", "--tags", "--progress"].map(String::from).to_vec();
    network_op(app, &state, Some(repo), args).await.map(|_| ())
}

#[tauri::command]
pub async fn pull(app: AppHandle, state: State<'_, AppState>, repo: String) -> Result<()> {
    let st = status(repo.clone()).await?;
    if st.branch.is_none() {
        return Err(Error::Other("You're not on a branch (detached HEAD), so there's nothing to pull into.".into()));
    }
    if st.upstream.is_none() {
        return Err(Error::Other("This branch isn't tracking a remote branch yet. Push it first.".into()));
    }
    let mode = match state.store.get().pull_mode.as_str() {
        "rebase" => "--rebase",
        "ff-only" => "--ff-only",
        _ => "--no-rebase",
    };
    let args = vec!["pull".into(), "--progress".into(), mode.into(), "--no-edit".into()];
    network_op(app, &state, Some(repo), args).await.map(|_| ())
}

#[tauri::command]
pub async fn push(app: AppHandle, state: State<'_, AppState>, repo: String, force: bool) -> Result<()> {
    let st = status(repo.clone()).await?;
    let branch = st
        .branch
        .ok_or_else(|| Error::Other("You're not on a branch (detached HEAD). Create a branch first.".into()))?;
    let mut args: Vec<String> = vec!["push".into(), "--progress".into()];
    if force {
        args.push("--force-with-lease".into());
    }
    if st.upstream.is_none() {
        let rs = remotes(repo.clone()).await?;
        let remote = rs
            .iter()
            .find(|r| r.name == "origin")
            .or(rs.first())
            .ok_or_else(|| Error::Other("This repository has no remotes to push to.".into()))?;
        args.extend(["-u".into(), remote.name.clone(), branch]);
    }
    network_op(app, &state, Some(repo), args).await.map(|_| ())
}

#[tauri::command]
pub async fn push_tag(app: AppHandle, state: State<'_, AppState>, repo: String, name: String) -> Result<()> {
    let rs = remotes(repo.clone()).await?;
    let remote = rs.iter().find(|r| r.name == "origin").or(rs.first())
        .ok_or_else(|| Error::Other("This repository has no remotes to push to.".into()))?
        .name
        .clone();
    let args = vec!["push".into(), "--progress".into(), remote, format!("refs/tags/{name}")];
    network_op(app, &state, Some(repo), args).await.map(|_| ())
}

// ---------------------------------------------------------------- accounts

#[tauri::command]
pub async fn auth_device_start(state: State<'_, AppState>, provider: String, host: Option<String>) -> Result<DeviceStart> {
    let settings = state.store.get();
    auth::device_start(&provider, host, &settings).await
}

#[tauri::command]
pub async fn auth_device_poll(state: State<'_, AppState>, start: DeviceStart) -> Result<Account> {
    let settings = state.store.get();
    let generation = state.auth_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let token = auth::device_poll(&start, &settings, || state.auth_generation.load(Ordering::SeqCst) != generation).await?;
    let account = auth::identify(&start.provider, &start.host, &token.access_token, None).await?;
    save_account(&state, account, token)
}

#[tauri::command]
pub fn auth_cancel(state: State<AppState>) {
    state.auth_generation.fetch_add(1, Ordering::SeqCst);
}

/// Sign in with a personal access token (or Bitbucket API token + email).
#[tauri::command]
pub async fn auth_token(
    state: State<'_, AppState>,
    provider: String,
    host: Option<String>,
    token: String,
    login: Option<String>,
) -> Result<Account> {
    let token = token.trim().to_string();
    let host = host.filter(|h| !h.trim().is_empty()).unwrap_or_else(|| {
        match provider.as_str() {
            "github" => "github.com",
            "gitlab" => "gitlab.com",
            _ => "bitbucket.org",
        }
        .into()
    });
    let account = auth::identify(&provider, &host, &token, login.as_deref()).await?;
    let tok = auth::Token {
        access_token: token,
        refresh_token: None,
        expires_at: None,
        client_id: None,
        login: login.map(|l| l.trim().to_string()),
    };
    save_account(&state, account, tok)
}

fn save_account(state: &AppState, account: Account, token: auth::Token) -> Result<Account> {
    state.tokens.save(&account.id, &token)?;
    let acc = account.clone();
    state.store.update(move |s| {
        s.accounts.retain(|a| a.id != acc.id);
        s.accounts.push(acc);
    })?;
    Ok(account)
}

#[tauri::command]
pub fn auth_sign_out(state: State<AppState>, id: String) -> Result<Settings> {
    state.tokens.delete(&id);
    state.store.update(|s| s.accounts.retain(|a| a.id != id))
}

#[tauri::command]
pub async fn auth_list_repos(state: State<'_, AppState>, id: String) -> Result<Vec<RemoteRepo>> {
    let account = state
        .store
        .get()
        .accounts
        .into_iter()
        .find(|a| a.id == id)
        .ok_or_else(|| Error::Auth("Unknown account".into()))?;
    let token = auth::access_token(&state.tokens, &account).await?;
    let login = state.tokens.load(&account.id).and_then(|t| t.login);
    auth::list_repos(&account, &token, login.as_deref()).await
}
