//! Sign-in for GitHub, GitLab and Bitbucket.
//!
//! GitHub and GitLab use the OAuth 2.0 device authorization grant: the app
//! shows a short code, the user approves it in their browser, and no client
//! secret ever ships inside the binary. Bitbucket Cloud has no device flow,
//! so it signs in with an Atlassian API token instead. Any provider also
//! accepts a personal access token.
//!
//! Tokens are stored in the OS keychain (Secret Service / Keychain /
//! Credential Manager), falling back to a user-only file when no keychain
//! is available (e.g. a bare window manager without gnome-keyring).

use crate::error::{Error, Result};
use crate::store::{Account, Settings};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const KEYRING_SERVICE: &str = "gitout";
const USER_AGENT: &str = concat!("GitOut/", env!("CARGO_PKG_VERSION"));

// Baked in at build time from CI secrets; overridable in Settings.
const GITHUB_CLIENT_ID: Option<&str> = option_env!("GITOUT_GITHUB_CLIENT_ID");
const GITLAB_CLIENT_ID: Option<&str> = option_env!("GITOUT_GITLAB_CLIENT_ID");

const GITHUB_SCOPES: &str = "repo read:user workflow";
const GITLAB_SCOPES: &str = "api read_user write_repository";

pub fn http() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .timeout(Duration::from_secs(30))
        .build()
        .expect("http client")
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

fn normalize_host(host: &str) -> String {
    host.trim()
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_lowercase()
}

fn default_host(provider: &str) -> &'static str {
    match provider {
        "github" => "github.com",
        "gitlab" => "gitlab.com",
        _ => "bitbucket.org",
    }
}

fn github_api(host: &str) -> String {
    if host == "github.com" { "https://api.github.com".into() } else { format!("https://{host}/api/v3") }
}

fn client_id(provider: &str, settings: &Settings) -> Result<String> {
    let (over, built) = match provider {
        "github" => (&settings.oauth.github_client_id, GITHUB_CLIENT_ID),
        "gitlab" => (&settings.oauth.gitlab_client_id, GITLAB_CLIENT_ID),
        _ => return Err(Error::Auth(format!("{provider} does not support browser sign-in"))),
    };
    // CI sets the build-time env var to "" when the repository variable is
    // missing, so an empty built-in ID counts as not configured.
    fn nonempty(s: &str) -> Option<&str> {
        Some(s.trim()).filter(|s| !s.is_empty())
    }
    over.as_deref()
        .and_then(nonempty)
        .or_else(|| built.and_then(nonempty))
        .map(str::to_string)
        .ok_or_else(|| {
            Error::Auth(format!(
                "No {provider} OAuth app is configured for this build. Add a client ID under \
                 Settings → Advanced, or sign in with a personal access token."
            ))
        })
}

// ---------------------------------------------------------------- token storage

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub access_token: String,
    #[serde(default)]
    pub refresh_token: Option<String>,
    /// Unix seconds; None means the token does not expire.
    #[serde(default)]
    pub expires_at: Option<i64>,
    /// Client ID used to obtain the token, needed to refresh it.
    #[serde(default)]
    pub client_id: Option<String>,
    /// Extra login identity some APIs need (Bitbucket: Atlassian account email).
    #[serde(default)]
    pub login: Option<String>,
}

pub struct TokenStore {
    fallback: PathBuf,
}

impl TokenStore {
    pub fn new(config_dir: &Path) -> Self {
        Self { fallback: config_dir.join("tokens.json") }
    }

    fn read_fallback(&self) -> std::collections::HashMap<String, Token> {
        std::fs::read_to_string(&self.fallback)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn write_fallback(&self, map: &std::collections::HashMap<String, Token>) -> Result<()> {
        if let Some(d) = self.fallback.parent() {
            std::fs::create_dir_all(d)?;
        }
        std::fs::write(&self.fallback, serde_json::to_vec(map)?)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&self.fallback, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(())
    }

    pub fn save(&self, id: &str, token: &Token) -> Result<()> {
        let json = serde_json::to_string(token)?;
        match keyring::Entry::new(KEYRING_SERVICE, id).and_then(|e| e.set_password(&json)) {
            Ok(()) => Ok(()),
            Err(e) => {
                eprintln!("keychain unavailable ({e}); storing token in user-only file");
                let mut map = self.read_fallback();
                map.insert(id.into(), token.clone());
                self.write_fallback(&map)
            }
        }
    }

    pub fn load(&self, id: &str) -> Option<Token> {
        keyring::Entry::new(KEYRING_SERVICE, id)
            .and_then(|e| e.get_password())
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .or_else(|| self.read_fallback().remove(id))
    }

    pub fn delete(&self, id: &str) {
        if let Ok(e) = keyring::Entry::new(KEYRING_SERVICE, id) {
            let _ = e.delete_credential();
        }
        let mut map = self.read_fallback();
        if map.remove(id).is_some() {
            let _ = self.write_fallback(&map);
        }
    }
}

/// Return a usable access token for `account`, refreshing it first if it is
/// about to expire (GitLab OAuth tokens last two hours).
pub async fn access_token(tokens: &TokenStore, account: &Account) -> Result<String> {
    let mut tok = tokens
        .load(&account.id)
        .ok_or_else(|| Error::Auth(format!("No saved credentials for {}; please sign in again", account.username)))?;
    let expiring = tok.expires_at.is_some_and(|t| t - 60 < now());
    if expiring && account.provider == "gitlab" {
        if let (Some(refresh), Some(cid)) = (tok.refresh_token.clone(), tok.client_id.clone()) {
            let resp: TokenResponse = http()
                .post(format!("https://{}/oauth/token", account.host))
                .form(&[("grant_type", "refresh_token"), ("refresh_token", &refresh), ("client_id", &cid)])
                .send()
                .await?
                .json()
                .await?;
            let access = resp.access_token.ok_or_else(|| {
                Error::Auth(format!("GitLab session for {} expired; please sign in again", account.username))
            })?;
            tok = Token {
                access_token: access,
                refresh_token: resp.refresh_token.or(Some(refresh)),
                expires_at: resp.expires_in.map(|e| now() + e),
                client_id: Some(cid),
                login: tok.login.clone(),
            };
            tokens.save(&account.id, &tok)?;
        }
    }
    Ok(tok.access_token)
}

/// Environment that makes git send each signed-in account's token to its host
/// over HTTPS. Uses GIT_CONFIG_* env vars (git ≥ 2.31) so tokens never appear
/// in process arguments, and only applies to URLs under that exact host.
pub async fn git_auth_env(tokens: &TokenStore, accounts: &[Account]) -> Vec<(String, String)> {
    let mut env = vec![];
    let mut seen = std::collections::HashSet::new();
    for acc in accounts {
        if !seen.insert(acc.host.clone()) {
            continue;
        }
        let Ok(token) = access_token(tokens, acc).await else { continue };
        let basic = base64::engine::general_purpose::STANDARD.encode(format!("{}:{}", acc.git_user, token));
        let i = env.len() / 2;
        env.push((format!("GIT_CONFIG_KEY_{i}"), format!("http.https://{}/.extraHeader", acc.host)));
        env.push((format!("GIT_CONFIG_VALUE_{i}"), format!("Authorization: Basic {basic}")));
    }
    if !env.is_empty() {
        env.push(("GIT_CONFIG_COUNT".into(), (env.len() / 2).to_string()));
    }
    env
}

// ---------------------------------------------------------------- device flow

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceStart {
    pub provider: String,
    pub host: String,
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub verification_uri_complete: Option<String>,
    pub interval: u64,
    pub expires_in: u64,
}

#[derive(Debug, Deserialize)]
struct DeviceCodeResponse {
    device_code: Option<String>,
    user_code: Option<String>,
    verification_uri: Option<String>,
    verification_uri_complete: Option<String>,
    interval: Option<u64>,
    expires_in: Option<u64>,
    error: Option<String>,
    error_description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct TokenResponse {
    access_token: Option<String>,
    refresh_token: Option<String>,
    expires_in: Option<i64>,
    error: Option<String>,
    error_description: Option<String>,
}

pub async fn device_start(provider: &str, host: Option<String>, settings: &Settings) -> Result<DeviceStart> {
    let host = normalize_host(host.as_deref().unwrap_or(default_host(provider)));
    let cid = client_id(provider, settings)?;
    let (url, scopes) = match provider {
        "github" => (format!("https://{host}/login/device/code"), GITHUB_SCOPES),
        "gitlab" => (format!("https://{host}/oauth/authorize_device"), GITLAB_SCOPES),
        _ => return Err(Error::Auth(format!("{provider} does not support browser sign-in"))),
    };
    let r: DeviceCodeResponse = http()
        .post(url)
        .header("Accept", "application/json")
        .form(&[("client_id", cid.as_str()), ("scope", scopes)])
        .send()
        .await?
        .json()
        .await?;
    if let Some(e) = r.error {
        return Err(Error::Auth(r.error_description.unwrap_or(e)));
    }
    Ok(DeviceStart {
        provider: provider.into(),
        host,
        device_code: r.device_code.ok_or_else(|| Error::Auth("missing device_code".into()))?,
        user_code: r.user_code.ok_or_else(|| Error::Auth("missing user_code".into()))?,
        verification_uri: r.verification_uri.ok_or_else(|| Error::Auth("missing verification_uri".into()))?,
        verification_uri_complete: r.verification_uri_complete,
        interval: r.interval.unwrap_or(5).max(1),
        expires_in: r.expires_in.unwrap_or(900),
    })
}

/// Poll until the user approves (or denies) the device code. `cancelled`
/// is checked between polls so the UI can abort a pending sign-in.
pub async fn device_poll(
    start: &DeviceStart,
    settings: &Settings,
    cancelled: impl Fn() -> bool,
) -> Result<Token> {
    let cid = client_id(&start.provider, settings)?;
    let url = match start.provider.as_str() {
        "github" => format!("https://{}/login/oauth/access_token", start.host),
        _ => format!("https://{}/oauth/token", start.host),
    };
    let deadline = now() + start.expires_in as i64;
    let mut interval = start.interval;
    loop {
        tokio::time::sleep(Duration::from_secs(interval)).await;
        if cancelled() {
            return Err(Error::Auth("Sign-in cancelled".into()));
        }
        if now() > deadline {
            return Err(Error::Auth("The sign-in code expired. Please try again.".into()));
        }
        let r: TokenResponse = http()
            .post(&url)
            .header("Accept", "application/json")
            .form(&[
                ("client_id", cid.as_str()),
                ("device_code", start.device_code.as_str()),
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ])
            .send()
            .await?
            .json()
            .await?;
        if let Some(access) = r.access_token {
            return Ok(Token {
                access_token: access,
                refresh_token: r.refresh_token,
                expires_at: r.expires_in.map(|e| now() + e),
                client_id: Some(cid),
                login: None,
            });
        }
        match r.error.as_deref() {
            Some("authorization_pending") => {}
            Some("slow_down") => interval += 5,
            Some("access_denied") => return Err(Error::Auth("Sign-in was denied in the browser".into())),
            Some("expired_token") => return Err(Error::Auth("The sign-in code expired. Please try again.".into())),
            Some(e) => return Err(Error::Auth(r.error_description.unwrap_or_else(|| e.to_string()))),
            None => return Err(Error::Auth("Unexpected response from the sign-in server".into())),
        }
    }
}

// ---------------------------------------------------------------- user + repos

#[derive(Deserialize)]
struct GhUser {
    login: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct GlUser {
    username: String,
    name: Option<String>,
    avatar_url: Option<String>,
}

#[derive(Deserialize)]
struct BbUser {
    username: Option<String>,
    nickname: Option<String>,
    display_name: Option<String>,
    links: Option<serde_json::Value>,
}

async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    if status == reqwest::StatusCode::UNAUTHORIZED || status == reqwest::StatusCode::FORBIDDEN {
        return Err(Error::Auth("The token was rejected. Check that it is valid and has the required scopes.".into()));
    }
    let body = resp.text().await.unwrap_or_default();
    Err(Error::Auth(format!("Server returned {status}: {}", body.chars().take(200).collect::<String>())))
}

fn bitbucket_basic(email: &str, token: &str) -> String {
    format!("Basic {}", base64::engine::general_purpose::STANDARD.encode(format!("{email}:{token}")))
}

/// Look up who a token belongs to and build the Account record for it.
/// For Bitbucket, `login` is the Atlassian account email used for API calls.
pub async fn identify(provider: &str, host: &str, token: &str, login: Option<&str>) -> Result<Account> {
    let host = normalize_host(host);
    let c = http();
    let (username, display_name, avatar_url, git_user) = match provider {
        "github" => {
            let u: GhUser = check(c.get(format!("{}/user", github_api(&host))).bearer_auth(token).send().await?)
                .await?
                .json()
                .await?;
            (u.login, u.name, u.avatar_url, "x-access-token".to_string())
        }
        "gitlab" => {
            let u: GlUser = check(c.get(format!("https://{host}/api/v4/user")).bearer_auth(token).send().await?)
                .await?
                .json()
                .await?;
            (u.username, u.name, u.avatar_url, "oauth2".to_string())
        }
        "bitbucket" => {
            let email = login.filter(|l| !l.trim().is_empty()).ok_or_else(|| {
                Error::Auth("Bitbucket needs your Atlassian account email along with the API token".into())
            })?;
            let u: BbUser = check(
                c.get("https://api.bitbucket.org/2.0/user")
                    .header("Authorization", bitbucket_basic(email.trim(), token))
                    .send()
                    .await?,
            )
            .await?
            .json()
            .await?;
            let avatar = u
                .links
                .as_ref()
                .and_then(|l| l.pointer("/avatar/href"))
                .and_then(|v| v.as_str())
                .map(String::from);
            let name = u.username.or(u.nickname).unwrap_or_else(|| email.to_string());
            // Bitbucket API tokens authenticate git over HTTPS with this fixed username.
            (name, u.display_name, avatar, "x-bitbucket-api-token-auth".to_string())
        }
        _ => return Err(Error::Auth(format!("Unknown provider {provider}"))),
    };
    let id = format!("{provider}:{host}:{username}");
    Ok(Account { id, provider: provider.into(), host, username, display_name, avatar_url, git_user })
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RemoteRepo {
    pub full_name: String,
    pub description: Option<String>,
    pub private: bool,
    pub https_url: String,
    pub ssh_url: Option<String>,
}

#[derive(Deserialize)]
struct GhRepo {
    full_name: String,
    description: Option<String>,
    private: bool,
    clone_url: String,
    ssh_url: Option<String>,
}

#[derive(Deserialize)]
struct GlRepo {
    path_with_namespace: String,
    description: Option<String>,
    visibility: Option<String>,
    http_url_to_repo: String,
    ssh_url_to_repo: Option<String>,
}

/// Repositories the account can access, most recently active first.
pub async fn list_repos(account: &Account, token: &str, bitbucket_email: Option<&str>) -> Result<Vec<RemoteRepo>> {
    let c = http();
    match account.provider.as_str() {
        "github" => {
            let url = format!("{}/user/repos?per_page=100&sort=pushed", github_api(&account.host));
            let repos: Vec<GhRepo> = check(c.get(url).bearer_auth(token).send().await?).await?.json().await?;
            Ok(repos
                .into_iter()
                .map(|r| RemoteRepo {
                    full_name: r.full_name,
                    description: r.description,
                    private: r.private,
                    https_url: r.clone_url,
                    ssh_url: r.ssh_url,
                })
                .collect())
        }
        "gitlab" => {
            let url = format!(
                "https://{}/api/v4/projects?membership=true&order_by=last_activity_at&per_page=100&simple=true",
                account.host
            );
            let repos: Vec<GlRepo> = check(c.get(url).bearer_auth(token).send().await?).await?.json().await?;
            Ok(repos
                .into_iter()
                .map(|r| RemoteRepo {
                    full_name: r.path_with_namespace,
                    description: r.description,
                    private: r.visibility.as_deref() != Some("public"),
                    https_url: r.http_url_to_repo,
                    ssh_url: r.ssh_url_to_repo,
                })
                .collect())
        }
        "bitbucket" => {
            let email = bitbucket_email.unwrap_or_default();
            let v: serde_json::Value = check(
                c.get("https://api.bitbucket.org/2.0/repositories?role=member&sort=-updated_on&pagelen=100")
                    .header("Authorization", bitbucket_basic(email, token))
                    .send()
                    .await?,
            )
            .await?
            .json()
            .await?;
            let mut out = vec![];
            for r in v["values"].as_array().into_iter().flatten() {
                let clones = r["links"]["clone"].as_array().cloned().unwrap_or_default();
                let find = |name: &str| {
                    clones.iter().find(|c| c["name"] == name).and_then(|c| c["href"].as_str()).map(String::from)
                };
                let Some(https) = find("https") else { continue };
                // Strip the "user@" Bitbucket embeds; our auth header supplies credentials.
                let https = url::Url::parse(&https)
                    .map(|mut u| {
                        let _ = u.set_username("");
                        u.to_string()
                    })
                    .unwrap_or(https);
                out.push(RemoteRepo {
                    full_name: r["full_name"].as_str().unwrap_or_default().into(),
                    description: r["description"].as_str().filter(|s| !s.is_empty()).map(String::from),
                    private: r["is_private"].as_bool().unwrap_or(true),
                    https_url: https,
                    ssh_url: find("ssh"),
                });
            }
            Ok(out)
        }
        p => Err(Error::Auth(format!("Unknown provider {p}"))),
    }
}
