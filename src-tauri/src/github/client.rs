use super::types::{ApiRun, Job, JobsPage, Repo, RunsPage, User, Workflow, WorkflowsPage};
use crate::model::{RunSnapshot, WatchedWorkflow};
use crate::progress::StepCount;
use bytes::Bytes;
use chrono::{DateTime, Utc};
use reqwest::header::{ACCEPT, AUTHORIZATION, HeaderMap, IF_NONE_MATCH, USER_AGENT};
use reqwest::{StatusCode, Url};
use serde::de::DeserializeOwned;
use std::collections::HashMap;
use std::future::Future;
use std::sync::{Mutex, RwLock};
use std::time::Duration;

pub const GITHUB_API: &str = "https://api.github.com";
/// Bounds every request so a socket dropped during sleep or a network change can't hang polling.
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Clone, Debug, PartialEq)]
pub enum ApiError {
    Unauthorized,
    Sso { url: Option<String> },
    RateLimited { reset_at: DateTime<Utc> },
    NotFound,
    Http(u16),
    Network(String),
}

impl ApiError {
    /// Short Spanish message for cards and banners.
    pub fn message(&self) -> String {
        match self {
            ApiError::Unauthorized => "Token inválido o caducado".into(),
            ApiError::Sso { .. } => "Requiere autorizar el token para la organización (SSO)".into(),
            ApiError::RateLimited { reset_at } => {
                format!(
                    "Límite de la API alcanzado, se reanuda a las {}",
                    reset_at.with_timezone(&chrono::Local).format("%H:%M")
                )
            }
            ApiError::NotFound => "No encontrado (¿repo privado sin acceso?)".into(),
            ApiError::Http(code) => format!("Error de GitHub ({code})"),
            ApiError::Network(_) => "Sin conexión con GitHub".into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RateInfo {
    pub remaining: Option<u32>,
    pub reset_at: Option<DateTime<Utc>>,
}

pub trait GithubApi {
    fn me(&self) -> impl Future<Output = Result<String, ApiError>> + Send;
    fn list_repos(&self) -> impl Future<Output = Result<Vec<Repo>, ApiError>> + Send;
    fn get_repo(&self, full: &str) -> impl Future<Output = Result<Repo, ApiError>> + Send;
    fn list_workflows(
        &self,
        repo: &str,
    ) -> impl Future<Output = Result<Vec<Workflow>, ApiError>> + Send;
    fn latest_run(
        &self,
        w: &WatchedWorkflow,
    ) -> impl Future<Output = Result<Option<RunSnapshot>, ApiError>> + Send;
    fn recent_success_durations(
        &self,
        w: &WatchedWorkflow,
    ) -> impl Future<Output = Result<Vec<u64>, ApiError>> + Send;
    fn jobs(
        &self,
        repo: &str,
        run_id: u64,
    ) -> impl Future<Output = Result<Vec<Job>, ApiError>> + Send;
    fn rate(&self) -> RateInfo;
}

struct Cached {
    etag: String,
    body: Bytes,
    next: Option<String>,
}

pub struct HttpClient {
    http: reqwest::Client,
    base: String,
    token: RwLock<Option<String>>,
    etags: Mutex<HashMap<String, Cached>>,
    rate: Mutex<RateInfo>,
}

fn header_str<'a>(h: &'a HeaderMap, name: &str) -> Option<&'a str> {
    h.get(name).and_then(|v| v.to_str().ok())
}

fn next_link(h: &HeaderMap) -> Option<String> {
    header_str(h, "link")?
        .split(',')
        .find(|part| part.contains("rel=\"next\""))
        .and_then(|part| Some(part[part.find('<')? + 1..part.find('>')?].to_string()))
}

impl HttpClient {
    pub fn new(base: &str, token: Option<String>) -> Self {
        Self::with_timeout(base, token, REQUEST_TIMEOUT)
    }

    pub fn with_timeout(base: &str, token: Option<String>, timeout: Duration) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(timeout.min(Duration::from_secs(10)))
            .timeout(timeout)
            .pool_idle_timeout(Duration::from_secs(60))
            .tcp_keepalive(Duration::from_secs(30))
            .build()
            .expect("http client");
        Self {
            http,
            base: base.trim_end_matches('/').to_string(),
            token: RwLock::new(token),
            etags: Mutex::new(HashMap::new()),
            rate: Mutex::new(RateInfo::default()),
        }
    }

    pub fn set_token(&self, token: Option<String>) {
        *self.token.write().unwrap() = token;
        // Cached bodies may belong to another identity.
        self.etags.lock().unwrap().clear();
    }

    pub fn has_token(&self) -> bool {
        self.token.read().unwrap().is_some()
    }

    fn url(&self, path: &str, query: &[(&str, &str)]) -> Url {
        let mut url = Url::parse(&format!("{}{}", self.base, path)).expect("valid api url");
        if !query.is_empty() {
            url.query_pairs_mut().extend_pairs(query);
        }
        url
    }

    fn record_rate(&self, h: &HeaderMap) {
        let mut rate = self.rate.lock().unwrap();
        if let Some(r) = header_str(h, "x-ratelimit-remaining").and_then(|v| v.parse().ok()) {
            rate.remaining = Some(r);
        }
        if let Some(t) = header_str(h, "x-ratelimit-reset").and_then(|v| v.parse().ok()) {
            rate.reset_at = DateTime::from_timestamp(t, 0);
        }
    }

    fn error_for(&self, status: StatusCode, h: &HeaderMap) -> ApiError {
        match status.as_u16() {
            401 => ApiError::Unauthorized,
            403 if h.contains_key("x-github-sso") => ApiError::Sso {
                url: header_str(h, "x-github-sso")
                    .and_then(|v| v.split("url=").nth(1))
                    .map(|u| u.trim().to_string()),
            },
            403 | 429
                if header_str(h, "x-ratelimit-remaining") == Some("0")
                    || h.contains_key("retry-after") =>
            {
                let retry = header_str(h, "retry-after").and_then(|v| v.parse::<i64>().ok());
                let reset_at = match retry {
                    Some(secs) => Utc::now() + chrono::Duration::seconds(secs),
                    None => self.rate.lock().unwrap().reset_at.unwrap_or_else(Utc::now),
                };
                ApiError::RateLimited { reset_at }
            }
            404 => ApiError::NotFound,
            code => ApiError::Http(code),
        }
    }

    /// GET with conditional request: a 304 reuses the cached body (and doesn't count
    /// against the rate limit).
    async fn get_raw(&self, url: &Url) -> Result<(Bytes, Option<String>), ApiError> {
        let key = url.to_string();
        let mut req = self
            .http
            .get(url.clone())
            .header(ACCEPT, "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header(USER_AGENT, "actions-lights");
        if let Some(t) = self.token.read().unwrap().as_deref() {
            req = req.header(AUTHORIZATION, format!("Bearer {t}"));
        }
        if let Some(c) = self.etags.lock().unwrap().get(&key) {
            req = req.header(IF_NONE_MATCH, c.etag.clone());
        }
        let resp = req
            .send()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;
        let headers = resp.headers().clone();
        self.record_rate(&headers);
        let status = resp.status();

        if status == StatusCode::NOT_MODIFIED {
            if let Some(c) = self.etags.lock().unwrap().get(&key) {
                return Ok((c.body.clone(), c.next.clone()));
            }
            return Err(ApiError::Http(304));
        }
        if !status.is_success() {
            return Err(self.error_for(status, &headers));
        }
        let body = resp
            .bytes()
            .await
            .map_err(|e| ApiError::Network(e.to_string()))?;
        let next = next_link(&headers);
        if let Some(etag) = header_str(&headers, "etag") {
            self.etags.lock().unwrap().insert(
                key,
                Cached {
                    etag: etag.to_string(),
                    body: body.clone(),
                    next: next.clone(),
                },
            );
        }
        Ok((body, next))
    }

    async fn get_json<T: DeserializeOwned>(&self, url: &Url) -> Result<T, ApiError> {
        let (body, _) = self.get_raw(url).await?;
        serde_json::from_slice(&body)
            .map_err(|e| ApiError::Network(format!("respuesta inválida: {e}")))
    }

    fn runs_url(&self, w: &WatchedWorkflow, extra: &[(&str, &str)]) -> Url {
        let mut q: Vec<(&str, &str)> = extra.to_vec();
        if let Some(b) = w.branch.as_deref() {
            q.push(("branch", b));
        }
        if let Some(e) = w.event.as_deref() {
            q.push(("event", e));
        }
        self.url(
            &format!("/repos/{}/actions/workflows/{}/runs", w.repo, w.workflow_id),
            &q,
        )
    }
}

impl GithubApi for HttpClient {
    async fn me(&self) -> Result<String, ApiError> {
        let u: User = self.get_json(&self.url("/user", &[])).await?;
        Ok(u.login)
    }

    async fn list_repos(&self) -> Result<Vec<Repo>, ApiError> {
        let mut out = Vec::new();
        let mut next = Some(self.url(
            "/user/repos",
            &[
                ("affiliation", "owner,collaborator,organization_member"),
                ("sort", "pushed"),
                ("per_page", "100"),
            ],
        ));
        while let Some(url) = next {
            let (body, link) = self.get_raw(&url).await?;
            let page: Vec<Repo> = serde_json::from_slice(&body)
                .map_err(|e| ApiError::Network(format!("respuesta inválida: {e}")))?;
            out.extend(page);
            next = link.and_then(|l| Url::parse(&l).ok());
        }
        Ok(out)
    }

    async fn get_repo(&self, full: &str) -> Result<Repo, ApiError> {
        self.get_json(&self.url(&format!("/repos/{full}"), &[]))
            .await
    }

    async fn list_workflows(&self, repo: &str) -> Result<Vec<Workflow>, ApiError> {
        let page: WorkflowsPage = self
            .get_json(&self.url(
                &format!("/repos/{repo}/actions/workflows"),
                &[("per_page", "100")],
            ))
            .await?;
        Ok(page.workflows)
    }

    async fn latest_run(&self, w: &WatchedWorkflow) -> Result<Option<RunSnapshot>, ApiError> {
        let page: RunsPage = self
            .get_json(&self.runs_url(w, &[("per_page", "1")]))
            .await?;
        Ok(page.workflow_runs.into_iter().next().map(Into::into))
    }

    async fn recent_success_durations(&self, w: &WatchedWorkflow) -> Result<Vec<u64>, ApiError> {
        let page: RunsPage = self
            .get_json(&self.runs_url(w, &[("status", "success"), ("per_page", "10")]))
            .await?;
        Ok(page
            .workflow_runs
            .into_iter()
            .map(|r: ApiRun| {
                let start = r.run_started_at.unwrap_or(r.created_at);
                (r.updated_at - start).num_seconds().max(0) as u64
            })
            .collect())
    }

    async fn jobs(&self, repo: &str, run_id: u64) -> Result<Vec<Job>, ApiError> {
        let page: JobsPage = self
            .get_json(&self.url(
                &format!("/repos/{repo}/actions/runs/{run_id}/jobs"),
                &[("per_page", "100")],
            ))
            .await?;
        Ok(page.jobs)
    }

    fn rate(&self) -> RateInfo {
        self.rate.lock().unwrap().clone()
    }
}

/// Steps across all jobs; `current` is the first step in progress.
pub fn steps_of(jobs: &[Job]) -> StepCount {
    let steps = jobs.iter().flat_map(|j| j.steps.iter());
    let total = steps.clone().count() as u32;
    let done = steps.clone().filter(|s| s.status == "completed").count() as u32;
    let current = steps
        .clone()
        .find(|s| s.status == "in_progress")
        .map(|s| s.name.clone());
    StepCount {
        done,
        total,
        current,
    }
}

pub fn failed_step(jobs: &[Job]) -> Option<String> {
    jobs.iter()
        .flat_map(|j| j.steps.iter())
        .find(|s| s.conclusion.as_deref() == Some("failure"))
        .map(|s| s.name.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::types::Step;
    use crate::model::NotifyPrefs;
    use mockito::Matcher;

    fn watched() -> WatchedWorkflow {
        WatchedWorkflow {
            repo: "acme/api".into(),
            workflow_id: 42,
            workflow_name: "Deploy".into(),
            branch: Some("release".into()),
            event: Some("push".into()),
            notify: NotifyPrefs::default(),
        }
    }

    const RUN_JSON: &str = r#"{"workflow_runs":[{"id":9,"run_attempt":2,"status":"in_progress","conclusion":null,
        "html_url":"https://github.com/acme/api/actions/runs/9","created_at":"2026-10-27T10:00:00Z",
        "run_started_at":"2026-10-27T10:00:05Z","updated_at":"2026-10-27T10:03:00Z",
        "head_branch":"release","event":"push"}]}"#;

    #[tokio::test]
    async fn latest_run_builds_query_and_parses() {
        let mut server = mockito::Server::new_async().await;
        let m = server
            .mock("GET", "/repos/acme/api/actions/workflows/42/runs")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("per_page".into(), "1".into()),
                Matcher::UrlEncoded("branch".into(), "release".into()),
                Matcher::UrlEncoded("event".into(), "push".into()),
            ]))
            .match_header("authorization", "Bearer t")
            .with_body(RUN_JSON)
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), Some("t".into()));
        let run = c.latest_run(&watched()).await.unwrap().unwrap();
        assert_eq!(
            (run.id, run.attempt, run.status.as_str()),
            (9, 2, "in_progress")
        );
        assert_eq!(
            (run.head_branch.as_deref(), run.event.as_deref()),
            (Some("release"), Some("push"))
        );
        m.assert_async().await;
    }

    #[tokio::test]
    async fn latest_run_empty_is_none() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock(
                "GET",
                Matcher::Regex("^/repos/acme/api/actions/workflows/42/runs".into()),
            )
            .with_body(r#"{"workflow_runs":[]}"#)
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), None);
        assert_eq!(c.latest_run(&watched()).await.unwrap(), None);
    }

    #[tokio::test]
    async fn etag_304_reuses_cached_body() {
        let mut server = mockito::Server::new_async().await;
        let path = Matcher::Regex("^/repos/acme/api/actions/workflows/42/runs".into());
        let first = server
            .mock("GET", path.clone())
            .with_header("etag", "\"abc\"")
            .with_body(RUN_JSON)
            .expect(1)
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), Some("t".into()));
        let a = c.latest_run(&watched()).await.unwrap();
        first.assert_async().await;
        first.remove_async().await;
        let second = server
            .mock("GET", path)
            .match_header("if-none-match", "\"abc\"")
            .with_status(304)
            .create_async()
            .await;
        let b = c.latest_run(&watched()).await.unwrap();
        second.assert_async().await;
        assert_eq!(a, b);
    }

    #[tokio::test]
    async fn hung_server_times_out_instead_of_waiting_forever() {
        // Accepts the connection but never answers (like a socket dropped during sleep).
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let mut held = Vec::new();
            loop {
                if let Ok((sock, _)) = listener.accept().await {
                    held.push(sock);
                }
            }
        });
        let c = HttpClient::with_timeout(
            &format!("http://{addr}"),
            None,
            std::time::Duration::from_millis(300),
        );
        let res = tokio::time::timeout(std::time::Duration::from_secs(5), c.me()).await;
        assert!(matches!(res, Ok(Err(ApiError::Network(_)))), "got {res:?}");
    }

    #[tokio::test]
    async fn unauthorized_maps() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/user")
            .with_status(401)
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), Some("bad".into()));
        assert_eq!(c.me().await, Err(ApiError::Unauthorized));
    }

    #[tokio::test]
    async fn sso_header_maps() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/user")
            .with_status(403)
            .with_header(
                "x-github-sso",
                "required; url=https://github.com/orgs/acme/sso?authorization_request=x",
            )
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), Some("t".into()));
        assert_eq!(
            c.me().await,
            Err(ApiError::Sso {
                url: Some("https://github.com/orgs/acme/sso?authorization_request=x".into())
            })
        );
    }

    #[tokio::test]
    async fn rate_limit_maps_and_records() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/user")
            .with_status(403)
            .with_header("x-ratelimit-remaining", "0")
            .with_header("x-ratelimit-reset", "1793145600")
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), Some("t".into()));
        let reset = DateTime::from_timestamp(1793145600, 0).unwrap();
        assert_eq!(c.me().await, Err(ApiError::RateLimited { reset_at: reset }));
        assert_eq!(
            c.rate(),
            RateInfo {
                remaining: Some(0),
                reset_at: Some(reset)
            }
        );
    }

    #[tokio::test]
    async fn not_found_maps() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/repos/acme/nope")
            .with_status(404)
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), None);
        assert_eq!(c.get_repo("acme/nope").await, Err(ApiError::NotFound));
    }

    #[tokio::test]
    async fn list_repos_follows_link_header() {
        let mut server = mockito::Server::new_async().await;
        let next = format!("<{}/user/repos?page=2>; rel=\"next\"", server.url());
        server
            .mock("GET", "/user/repos")
            .match_query(Matcher::UrlEncoded(
                "affiliation".into(),
                "owner,collaborator,organization_member".into(),
            ))
            .with_header("link", &next)
            .with_body(r#"[{"full_name":"acme/api","private":true,"default_branch":"main"}]"#)
            .create_async()
            .await;
        server
            .mock("GET", "/user/repos")
            .match_query(Matcher::UrlEncoded("page".into(), "2".into()))
            .with_body(r#"[{"full_name":"luis/blog","private":false,"default_branch":"master"}]"#)
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), Some("t".into()));
        let names: Vec<_> = c
            .list_repos()
            .await
            .unwrap()
            .into_iter()
            .map(|r| r.full_name)
            .collect();
        assert_eq!(names, vec!["acme/api", "luis/blog"]);
    }

    #[tokio::test]
    async fn durations_from_success_runs() {
        let mut server = mockito::Server::new_async().await;
        server
            .mock("GET", "/repos/acme/api/actions/workflows/42/runs")
            .match_query(Matcher::AllOf(vec![
                Matcher::UrlEncoded("status".into(), "success".into()),
                Matcher::UrlEncoded("per_page".into(), "10".into()),
            ]))
            .with_body(
                r#"{"workflow_runs":[
                {"id":1,"run_attempt":1,"status":"completed","conclusion":"success","html_url":"u","created_at":"2026-10-27T10:00:00Z","run_started_at":"2026-10-27T10:00:00Z","updated_at":"2026-10-27T10:05:00Z"},
                {"id":2,"run_attempt":1,"status":"completed","conclusion":"success","html_url":"u","created_at":"2026-10-27T09:00:00Z","run_started_at":"2026-10-27T09:00:00Z","updated_at":"2026-10-27T09:02:00Z"}]}"#,
            )
            .create_async()
            .await;
        let c = HttpClient::new(&server.url(), None);
        assert_eq!(
            c.recent_success_durations(&watched()).await.unwrap(),
            vec![300, 120]
        );
    }

    fn step(n: u32, name: &str, status: &str, conclusion: Option<&str>) -> Step {
        Step {
            name: name.into(),
            status: status.into(),
            conclusion: conclusion.map(Into::into),
            number: n,
        }
    }

    #[test]
    fn steps_and_failed_step() {
        let jobs = vec![
            Job {
                status: "completed".into(),
                conclusion: Some("failure".into()),
                steps: vec![
                    step(1, "Checkout", "completed", Some("success")),
                    step(2, "Run migrations", "completed", Some("failure")),
                    step(3, "Notify", "queued", None),
                ],
            },
            Job {
                status: "in_progress".into(),
                conclusion: None,
                steps: vec![
                    step(1, "Build", "in_progress", None),
                    step(2, "Push", "queued", None),
                ],
            },
        ];
        assert_eq!(
            steps_of(&jobs),
            StepCount {
                done: 2,
                total: 5,
                current: Some("Build".into())
            }
        );
        assert_eq!(failed_step(&jobs).as_deref(), Some("Run migrations"));
    }
}
