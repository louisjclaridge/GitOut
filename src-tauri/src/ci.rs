//! CI/CD runs (GitHub Actions workflow runs today; GitLab pipelines and Azure
//! DevOps builds later), read through the API of the signed-in account whose
//! host matches the repository's remote.
//!
//! Every provider is mapped onto one vocabulary so the UI never needs to know
//! which host it's talking to. Read-only: GitOut shows progress and links to
//! the run on the host for anything else.

use crate::auth;
use crate::error::{Error, Result};
use crate::releases::{self, ReleaseHost};
use serde::Serialize;
use serde_json::Value;

/// Hosts GitOut can read CI/CD runs from.
pub fn supported(provider: Option<&str>) -> bool {
    provider == Some("github")
}

/// One run of a pipeline (a GitHub workflow run, a GitLab pipeline, an Azure DevOps build).
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// Opaque to the UI; passed back to fetch the run's jobs.
    pub id: String,
    /// The pipeline's name, e.g. "CI".
    pub name: String,
    /// What the run is about: usually the commit subject or PR title.
    pub title: String,
    /// "queued" | "waiting" | "running" | "completed"
    pub status: String,
    /// Set once completed: "success" | "failure" | "cancelled" | "skipped" | "timed_out" | "action_required" | "neutral"
    pub conclusion: Option<String>,
    pub branch: Option<String>,
    pub head_sha: String,
    /// What started it, e.g. "push" or "pull request".
    pub trigger: String,
    /// The per-pipeline run number shown on the host, e.g. #42.
    pub number: u64,
    pub attempt: u64,
    pub actor: Option<String>,
    pub created_at: Option<String>,
    pub started_at: Option<String>,
    pub updated_at: Option<String>,
    pub url: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub number: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub url: Option<String>,
    pub steps: Vec<Step>,
}

fn s(v: &Value, key: &str) -> Option<String> {
    v[key].as_str().map(String::from)
}

// ---------------------------------------------------------------- GitHub Actions

fn github_status(v: &Value) -> String {
    match v["status"].as_str().unwrap_or_default() {
        "in_progress" => "running",
        "completed" => "completed",
        "waiting" => "waiting",
        // queued, requested, pending
        _ => "queued",
    }
    .into()
}

fn github_conclusion(v: &Value) -> Option<String> {
    let c = v["conclusion"].as_str()?;
    Some(
        match c {
            "startup_failure" => "failure",
            "stale" => "skipped",
            other => other,
        }
        .into(),
    )
}

fn github_run(v: &Value) -> Run {
    Run {
        id: v["id"].as_u64().map(|n| n.to_string()).unwrap_or_default(),
        name: s(v, "name").unwrap_or_default(),
        title: s(v, "display_title").unwrap_or_default(),
        status: github_status(v),
        conclusion: github_conclusion(v),
        branch: s(v, "head_branch"),
        head_sha: s(v, "head_sha").unwrap_or_default(),
        trigger: s(v, "event").unwrap_or_default().replace('_', " "),
        number: v["run_number"].as_u64().unwrap_or_default(),
        attempt: v["run_attempt"].as_u64().unwrap_or(1),
        actor: v["actor"]["login"].as_str().map(String::from),
        created_at: s(v, "created_at"),
        started_at: s(v, "run_started_at"),
        updated_at: s(v, "updated_at"),
        url: s(v, "html_url"),
    }
}

fn github_job(v: &Value) -> Job {
    Job {
        id: v["id"].as_u64().map(|n| n.to_string()).unwrap_or_default(),
        name: s(v, "name").unwrap_or_default(),
        status: github_status(v),
        conclusion: github_conclusion(v),
        started_at: s(v, "started_at"),
        completed_at: s(v, "completed_at"),
        url: s(v, "html_url"),
        steps: v["steps"]
            .as_array()
            .into_iter()
            .flatten()
            .map(|st| Step {
                number: st["number"].as_u64().unwrap_or_default(),
                name: s(st, "name").unwrap_or_default(),
                status: github_status(st),
                conclusion: github_conclusion(st),
                started_at: s(st, "started_at"),
                completed_at: s(st, "completed_at"),
            })
            .collect(),
    }
}

// ---------------------------------------------------------------- client

/// API client for one repository's CI/CD runs.
pub struct Ci {
    provider: String,
    base: String,
    token: String,
    http: reqwest::Client,
}

impl Ci {
    pub fn new(host: &ReleaseHost, token: String) -> Result<Self> {
        let provider = host.provider.clone().unwrap_or_default();
        let base = match provider.as_str() {
            "github" => format!("{}/repos/{}/actions", auth::github_api(&host.host), host.project),
            _ => return Err(Error::Other(format!("GitOut can't show CI/CD runs from {} yet.", host.host))),
        };
        Ok(Self { provider, base, token, http: auth::http() })
    }

    async fn get(&self, path: &str) -> Result<Value> {
        let mut req = self.http.get(format!("{}{path}", self.base)).bearer_auth(&self.token);
        if self.provider == "github" {
            req = req.header("Accept", "application/vnd.github+json").header("X-GitHub-Api-Version", "2022-11-28");
        }
        Ok(releases::check_with(req.send().await?, "see this project's CI/CD runs").await?.json().await?)
    }

    /// The most recent runs, newest first.
    pub async fn runs(&self) -> Result<Vec<Run>> {
        let v = self.get("/runs?per_page=30").await?;
        Ok(v["workflow_runs"].as_array().into_iter().flatten().map(github_run).collect())
    }

    /// The jobs of a run's latest attempt, with their steps.
    pub async fn jobs(&self, run_id: &str) -> Result<Vec<Job>> {
        let id: u64 = run_id.parse().map_err(|_| Error::Other(format!("Invalid run id {run_id}")))?;
        let v = self.get(&format!("/runs/{id}/jobs?per_page=100&filter=latest")).await?;
        Ok(v["jobs"].as_array().into_iter().flatten().map(github_job).collect())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_maps_to_common_vocabulary() {
        let v: Value = serde_json::from_str(
            r#"{"id":7,"name":"CI","display_title":"Fix bug","status":"in_progress","conclusion":null,
                "head_branch":"main","head_sha":"abc","event":"pull_request","run_number":12,"run_attempt":2,
                "actor":{"login":"ada"},"html_url":"https://github.com/a/b/actions/runs/7"}"#,
        )
        .unwrap();
        let r = github_run(&v);
        assert_eq!((r.id.as_str(), r.title.as_str(), r.status.as_str(), r.attempt), ("7", "Fix bug", "running", 2));
        assert_eq!(r.trigger, "pull request");
        assert_eq!(r.conclusion, None);

        let v: Value = serde_json::from_str(
            r#"{"id":1,"name":"build","status":"completed","conclusion":"startup_failure",
                "steps":[{"number":1,"name":"Set up job","status":"pending","conclusion":null}]}"#,
        )
        .unwrap();
        let j = github_job(&v);
        assert_eq!(j.conclusion.as_deref(), Some("failure"));
        assert_eq!(j.steps[0].status, "queued");
    }
}
