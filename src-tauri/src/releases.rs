//! Hosted releases on GitHub and GitLab, managed through the API of the
//! signed-in account whose host matches the repository's remote.
//!
//! Bitbucket has no releases, so it is reported as unsupported. Releases
//! always point at a tag; the UI creates and pushes the tag with git first,
//! so the API calls here never have to create tags on the server.

use crate::auth;
use crate::error::{Error, Result};
use crate::store::Account;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Where a remote URL points: `host` as accounts store it, `path` like "owner/repo".
#[derive(Debug, Clone, PartialEq)]
pub struct RemoteLocation {
    pub host: String,
    pub path: String,
}

/// Parse https://, ssh:// and scp-style (`git@host:owner/repo.git`) remote URLs.
pub fn parse_remote_url(url: &str) -> Option<RemoteLocation> {
    let url = url.trim();
    let (host, path) = if let Some((scheme, rest)) = url.split_once("://") {
        let (authority, path) = rest.split_once('/')?;
        let authority = authority.rsplit_once('@').map(|(_, h)| h).unwrap_or(authority);
        // An https port is part of the host (self-hosted GitLab on :8443); an ssh port isn't.
        let host = if scheme.starts_with("http") { authority } else { authority.split(':').next()? };
        (host.to_string(), path)
    } else {
        let (authority, path) = url.split_once(':')?;
        // "C:/src/repo" is a Windows path, and anything with a slash before the colon is local.
        if authority.len() <= 1 || authority.contains(['/', '\\']) {
            return None;
        }
        let host = authority.rsplit_once('@').map(|(_, h)| h).unwrap_or(authority);
        (host.to_string(), path)
    };
    let path = path.trim_matches('/');
    let path = path.strip_suffix(".git").unwrap_or(path).trim_matches('/');
    if host.is_empty() || !path.contains('/') {
        return None;
    }
    let host = match host.to_lowercase().as_str() {
        // GitHub's SSH-over-443 endpoint.
        "ssh.github.com" => "github.com".to_string(),
        h => h.to_string(),
    };
    Some(RemoteLocation { host, path: path.to_string() })
}

fn guess_provider(host: &str) -> Option<&'static str> {
    if host.contains("github") {
        Some("github")
    } else if host.contains("gitlab") {
        Some("gitlab")
    } else if host.contains("bitbucket") {
        Some("bitbucket")
    } else {
        None
    }
}

/// Where this repository's releases live, and whether GitOut can manage them.
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseHost {
    /// "github" | "gitlab" | "bitbucket", or None when the host isn't recognised.
    pub provider: Option<String>,
    pub host: String,
    /// "owner/repo" (GitLab: the full namespace path).
    pub project: String,
    /// The git remote tags are pushed to.
    pub remote: String,
    pub account_id: Option<String>,
    /// The releases page in a browser.
    pub web_url: Option<String>,
    /// GitHub has drafts and pre-releases; GitLab doesn't.
    pub supports_drafts: bool,
    /// Why releases can't be managed here, if they can't.
    pub problem: Option<String>,
}

impl ReleaseHost {
    pub fn unavailable(problem: impl Into<String>) -> Self {
        Self {
            provider: None,
            host: String::new(),
            project: String::new(),
            remote: String::new(),
            account_id: None,
            web_url: None,
            supports_drafts: false,
            problem: Some(problem.into()),
        }
    }
}

/// Work out the release host for a remote URL given the signed-in accounts.
pub fn resolve_host(remote: &str, url: &str, accounts: &[Account]) -> ReleaseHost {
    let Some(loc) = parse_remote_url(url) else {
        return ReleaseHost {
            remote: remote.into(),
            ..ReleaseHost::unavailable(format!("The {remote} remote ({url}) isn't a GitHub or GitLab URL."))
        };
    };
    let account = accounts.iter().find(|a| a.host == loc.host);
    let provider = account.map(|a| a.provider.as_str()).or_else(|| guess_provider(&loc.host));
    let web_url = match provider {
        Some("github") => Some(format!("https://{}/{}/releases", loc.host, loc.path)),
        Some("gitlab") => Some(format!("https://{}/{}/-/releases", loc.host, loc.path)),
        _ => None,
    };
    let problem = match (provider, account) {
        (Some("bitbucket"), _) => Some("Bitbucket doesn't have releases. Use tags instead.".to_string()),
        (Some(_), Some(_)) => None,
        (Some(_), None) => Some(format!("Sign in to {} under Settings → Accounts to manage its releases.", loc.host)),
        (None, _) => Some(format!(
            "{} isn't a GitHub or GitLab host GitOut knows about. Sign in to it under Settings → Accounts.",
            loc.host
        )),
    };
    ReleaseHost {
        provider: provider.map(String::from),
        host: loc.host,
        project: loc.path,
        remote: remote.into(),
        account_id: account.map(|a| a.id.clone()),
        web_url,
        supports_drafts: provider == Some("github"),
        problem,
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub name: String,
    pub url: String,
    pub size: Option<u64>,
    pub downloads: Option<u64>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Release {
    /// GitHub: numeric release id. GitLab: the tag name (its release key).
    pub id: String,
    pub tag: String,
    /// May be empty; the UI falls back to the tag.
    pub name: String,
    pub body: String,
    pub draft: bool,
    pub prerelease: bool,
    /// RFC 3339. None for unpublished drafts.
    pub published_at: Option<String>,
    pub created_at: Option<String>,
    pub author: Option<String>,
    pub url: Option<String>,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReleaseInput {
    pub tag: String,
    pub name: String,
    pub body: String,
    #[serde(default)]
    pub draft: bool,
    #[serde(default)]
    pub prerelease: bool,
}

fn s(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().map(String::from)
}

fn github_release(v: &Value) -> Release {
    Release {
        id: v["id"].as_u64().map(|n| n.to_string()).unwrap_or_default(),
        tag: s(v, "tag_name").unwrap_or_default(),
        name: s(v, "name").unwrap_or_default(),
        body: s(v, "body").unwrap_or_default(),
        draft: v["draft"].as_bool().unwrap_or(false),
        prerelease: v["prerelease"].as_bool().unwrap_or(false),
        published_at: s(v, "published_at"),
        created_at: s(v, "created_at"),
        author: v["author"]["login"].as_str().map(String::from),
        url: s(v, "html_url"),
        assets: v["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| Asset {
                name: s(a, "name").unwrap_or_default(),
                url: s(a, "browser_download_url").unwrap_or_default(),
                size: a["size"].as_u64(),
                downloads: a["download_count"].as_u64(),
            })
            .collect(),
    }
}

fn gitlab_release(v: &Value) -> Release {
    let tag = s(v, "tag_name").unwrap_or_default();
    Release {
        id: tag.clone(),
        tag,
        name: s(v, "name").unwrap_or_default(),
        body: s(v, "description").unwrap_or_default(),
        draft: false,
        prerelease: false,
        published_at: s(v, "released_at"),
        created_at: s(v, "created_at"),
        author: v["author"]["username"].as_str().map(String::from),
        url: v["_links"]["self"].as_str().map(String::from),
        assets: v["assets"]["links"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|a| Asset {
                name: s(a, "name").unwrap_or_default(),
                url: s(a, "direct_asset_url").or_else(|| s(a, "url")).unwrap_or_default(),
                size: None,
                downloads: None,
            })
            .collect(),
    }
}

/// Pull a readable message out of a GitHub or GitLab error body.
fn error_detail(v: &Value) -> Option<String> {
    let mut parts = vec![];
    match &v["message"] {
        Value::String(m) => parts.push(m.clone()),
        Value::Null => {}
        // GitLab validation errors: {"message": {"name": ["is too long"]}}
        other => parts.push(other.to_string()),
    }
    if let Some(e) = v["error"].as_str() {
        parts.push(e.to_string());
    }
    for e in v["errors"].as_array().into_iter().flatten() {
        let msg = match (e["field"].as_str(), e["code"].as_str()) {
            (Some("tag_name"), Some("already_exists")) => "a release for this tag already exists".to_string(),
            (field, code) => e["message"]
                .as_str()
                .map(String::from)
                .unwrap_or_else(|| format!("{} {}", field.unwrap_or_default(), code.unwrap_or_default()).trim().to_string()),
        };
        parts.push(msg);
    }
    (!parts.is_empty()).then(|| parts.join(": "))
}

async fn check(resp: reqwest::Response) -> Result<reqwest::Response> {
    let status = resp.status();
    if status.is_success() {
        return Ok(resp);
    }
    let text = resp.text().await.unwrap_or_default();
    let detail = serde_json::from_str::<Value>(&text)
        .ok()
        .and_then(|v| error_detail(&v))
        .unwrap_or_else(|| text.chars().take(200).collect());
    Err(Error::Other(match status.as_u16() {
        401 => "The server rejected your sign-in. Sign in again under Settings → Accounts.".into(),
        403 => format!("You don't have permission to manage releases in this project. ({detail})"),
        404 => format!("The project or release wasn't found, or your account can't see it. ({detail})"),
        _ => format!("Server returned {status}: {detail}"),
    }))
}

/// API client for one project's releases.
pub struct Releases {
    provider: String,
    base: String,
    token: String,
    http: reqwest::Client,
}

fn encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

impl Releases {
    pub fn new(host: &ReleaseHost, token: String) -> Result<Self> {
        let provider = host.provider.clone().unwrap_or_default();
        let base = match provider.as_str() {
            "github" => format!("{}/repos/{}", auth::github_api(&host.host), host.project),
            "gitlab" => format!("https://{}/api/v4/projects/{}", host.host, encode(&host.project)),
            _ => return Err(Error::Other(host.problem.clone().unwrap_or_else(|| "Releases aren't supported here".into()))),
        };
        Ok(Self { provider, base, token, http: auth::http() })
    }

    fn req(&self, method: reqwest::Method, path: &str) -> reqwest::RequestBuilder {
        let r = self.http.request(method, format!("{}{path}", self.base)).bearer_auth(&self.token);
        if self.provider == "github" {
            r.header("Accept", "application/vnd.github+json").header("X-GitHub-Api-Version", "2022-11-28")
        } else {
            r
        }
    }

    fn parse(&self, v: &Value) -> Release {
        if self.provider == "github" { github_release(v) } else { gitlab_release(v) }
    }

    pub async fn list(&self) -> Result<Vec<Release>> {
        let v: Value = check(self.req(reqwest::Method::GET, "/releases?per_page=100").send().await?).await?.json().await?;
        Ok(v.as_array().into_iter().flatten().map(|r| self.parse(r)).collect())
    }

    pub async fn create(&self, input: &ReleaseInput) -> Result<Release> {
        let body = if self.provider == "github" {
            json!({
                "tag_name": input.tag,
                "name": input.name,
                "body": input.body,
                "draft": input.draft,
                "prerelease": input.prerelease,
            })
        } else {
            json!({ "tag_name": input.tag, "name": input.name, "description": input.body })
        };
        let v: Value = check(self.req(reqwest::Method::POST, "/releases").json(&body).send().await?).await?.json().await?;
        Ok(self.parse(&v))
    }

    pub async fn update(&self, id: &str, input: &ReleaseInput) -> Result<Release> {
        let req = if self.provider == "github" {
            self.req(reqwest::Method::PATCH, &format!("/releases/{}", encode(id))).json(&json!({
                "name": input.name,
                "body": input.body,
                "draft": input.draft,
                "prerelease": input.prerelease,
            }))
        } else {
            self.req(reqwest::Method::PUT, &format!("/releases/{}", encode(id)))
                .json(&json!({ "name": input.name, "description": input.body }))
        };
        let v: Value = check(req.send().await?).await?.json().await?;
        Ok(self.parse(&v))
    }

    /// Deletes the release only; its tag stays.
    pub async fn delete(&self, id: &str) -> Result<()> {
        check(self.req(reqwest::Method::DELETE, &format!("/releases/{}", encode(id))).send().await?).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn loc(host: &str, path: &str) -> Option<RemoteLocation> {
        Some(RemoteLocation { host: host.into(), path: path.into() })
    }

    #[test]
    fn remote_urls() {
        assert_eq!(parse_remote_url("https://github.com/ada/engine.git"), loc("github.com", "ada/engine"));
        assert_eq!(parse_remote_url("https://user@github.com/ada/engine/"), loc("github.com", "ada/engine"));
        assert_eq!(parse_remote_url("git@github.com:ada/engine.git"), loc("github.com", "ada/engine"));
        assert_eq!(parse_remote_url("ssh://git@ssh.github.com:443/ada/engine.git"), loc("github.com", "ada/engine"));
        assert_eq!(parse_remote_url("ssh://git@gitlab.com:2222/group/sub/proj.git"), loc("gitlab.com", "group/sub/proj"));
        assert_eq!(parse_remote_url("https://git.example.com:8443/team/app"), loc("git.example.com:8443", "team/app"));
        assert_eq!(parse_remote_url("C:/src/repo"), None);
        assert_eq!(parse_remote_url("/home/ada/repo.git"), None);
        assert_eq!(parse_remote_url("file:///home/ada/repo.git"), None);
        assert_eq!(parse_remote_url("../other"), None);
    }

    #[test]
    fn host_problems() {
        let gh = Account {
            id: "github:github.com:ada".into(),
            provider: "github".into(),
            host: "github.com".into(),
            username: "ada".into(),
            display_name: None,
            avatar_url: None,
            git_user: "x-access-token".into(),
        };
        let h = resolve_host("origin", "git@github.com:ada/engine.git", &[gh]);
        assert!(h.problem.is_none());
        assert!(h.supports_drafts);
        assert_eq!(h.web_url.as_deref(), Some("https://github.com/ada/engine/releases"));

        let h = resolve_host("origin", "https://gitlab.com/g/p.git", &[]);
        assert_eq!(h.provider.as_deref(), Some("gitlab"));
        assert!(h.problem.unwrap().contains("Sign in"));

        let h = resolve_host("origin", "git@bitbucket.org:t/r.git", &[]);
        assert!(h.problem.unwrap().contains("Bitbucket"));
    }

    #[test]
    fn error_messages() {
        let v: Value = serde_json::from_str(
            r#"{"message":"Validation Failed","errors":[{"resource":"Release","code":"already_exists","field":"tag_name"}]}"#,
        )
        .unwrap();
        assert_eq!(error_detail(&v).unwrap(), "Validation Failed: a release for this tag already exists");
    }
}
