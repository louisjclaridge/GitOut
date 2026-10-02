//! Persistent, non-secret app settings stored as JSON in the OS config dir.
//! Tokens never live here; they go to the OS keychain (see `auth.rs`).

use crate::error::Result;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

const MAX_RECENT: usize = 15;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    /// "github" | "gitlab" | "bitbucket"
    pub provider: String,
    /// Host that remotes must match for this account's token to be used, e.g. "github.com".
    pub host: String,
    pub username: String,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    /// Username sent alongside the token for HTTPS git operations.
    pub git_user: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct OAuthOverrides {
    pub github_client_id: Option<String>,
    pub gitlab_client_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub recent_repos: Vec<String>,
    pub open_repos: Vec<String>,
    pub active_repo: Option<String>,
    pub accounts: Vec<Account>,
    /// "system" | "dark" | "light"
    pub theme: String,
    /// "merge" | "rebase" | "ff-only"
    pub pull_mode: String,
    pub auto_update: bool,
    pub oauth: OAuthOverrides,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent_repos: vec![],
            open_repos: vec![],
            active_repo: None,
            accounts: vec![],
            theme: "system".into(),
            pull_mode: "merge".into(),
            auto_update: true,
            oauth: OAuthOverrides::default(),
        }
    }
}

pub struct Store {
    path: PathBuf,
    pub settings: Mutex<Settings>,
}

impl Store {
    pub fn load(dir: PathBuf) -> Self {
        let path = dir.join("settings.json");
        let settings = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path, settings: Mutex::new(settings) }
    }

    pub fn get(&self) -> Settings {
        self.settings.lock().unwrap().clone()
    }

    /// Apply a mutation and persist the result.
    pub fn update<F: FnOnce(&mut Settings)>(&self, f: F) -> Result<Settings> {
        let mut s = self.settings.lock().unwrap();
        f(&mut s);
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&*s)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(s.clone())
    }

    pub fn touch_recent(&self, repo: &str) -> Result<Settings> {
        self.update(|s| {
            s.recent_repos.retain(|r| r != repo);
            s.recent_repos.insert(0, repo.to_string());
            s.recent_repos.truncate(MAX_RECENT);
        })
    }
}
