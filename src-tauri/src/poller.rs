use crate::github::{failed_step, steps_of, ApiError, GithubApi, Job, RateInfo};
use crate::model::{Card, Config, Light, RepoGroup, RunSnapshot, WatchedWorkflow};
use crate::notify_rules::{decide, NotifyKind, Seen};
use crate::progress::{estimate, median_secs, step_label};
use crate::status::{is_running, label_for, light_for};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Notification {
    pub watched: WatchedWorkflow,
    pub run: RunSnapshot,
    pub kind: NotifyKind,
    pub failed_step: Option<String>,
}

/// Memory carried between cycles, keyed by `WatchedWorkflow::key()`.
pub struct PollerState {
    seen: HashMap<String, Seen>,
    /// (run_id the median was fetched for, median)
    medians: HashMap<String, (u64, Option<u64>)>,
    /// ((run_id, attempt), failed step) so a failed run's jobs are fetched once
    failures: HashMap<String, ((u64, u32), Option<String>)>,
    first_poll: bool,
}

impl PollerState {
    pub fn new() -> Self {
        Self { seen: HashMap::new(), medians: HashMap::new(), failures: HashMap::new(), first_poll: true }
    }
}

pub struct CycleOutput {
    pub groups: Vec<RepoGroup>,
    pub notifications: Vec<Notification>,
    pub any_running: bool,
    pub min_running_progress: Option<f32>,
    pub auth_error: Option<ApiError>,
    pub requests: u32,
}

fn empty_card(w: &WatchedWorkflow) -> Card {
    Card {
        key: w.key(),
        repo: w.repo.clone(),
        workflow_name: w.workflow_name.clone(),
        branch: w.branch.clone(),
        light: Light::Gray,
        label: None,
        run_url: None,
        started_at: None,
        completed_at: None,
        progress: None,
        expected_secs: None,
        step_label: None,
        failed_step: None,
        error: None,
    }
}

pub async fn run_cycle<A: GithubApi>(api: &A, cfg: &Config, st: &mut PollerState, now: DateTime<Utc>) -> CycleOutput {
    let mut groups: Vec<RepoGroup> = Vec::new();
    let mut notifications = Vec::new();
    let mut auth_error = None;
    let mut requests = 0u32;
    let mut running_progress: Vec<f32> = Vec::new();
    let mut any_running = false;

    for w in &cfg.watched {
        let key = w.key();
        let mut card = empty_card(w);
        requests += 1;
        match api.latest_run(w).await {
            Err(e) => {
                if matches!(e, ApiError::Unauthorized | ApiError::Sso { .. }) && auth_error.is_none() {
                    auth_error = Some(e.clone());
                }
                card.error = Some(e.message());
            }
            Ok(None) => {}
            Ok(Some(run)) => {
                let running = is_running(&run.status);
                card.light = light_for(&run.status, run.conclusion.as_deref());
                card.label = label_for(&run.status);
                card.run_url = Some(run.html_url.clone());
                card.started_at = run.run_started_at.or(Some(run.created_at));
                if running {
                    any_running = true;
                    if st.medians.get(&key).map(|(id, _)| *id) != Some(run.id) {
                        requests += 1;
                        let median = api.recent_success_durations(w).await.ok().and_then(|d| median_secs(&d));
                        st.medians.insert(key.clone(), (run.id, median));
                    }
                    let median = st.medians.get(&key).and_then(|(_, m)| *m);
                    requests += 1;
                    let steps = api.jobs(&w.repo, run.id).await.ok().map(|j| steps_of(&j));
                    let elapsed = card.started_at.map(|s| (now - s).num_seconds().max(0) as u64).unwrap_or(0);
                    card.progress = estimate(elapsed, median, steps.as_ref());
                    card.expected_secs = median;
                    card.step_label = steps.as_ref().and_then(step_label);
                    running_progress.push(card.progress.unwrap_or(0.0));
                } else {
                    card.completed_at = Some(run.updated_at);
                    if card.light == Light::Red {
                        let id = (run.id, run.attempt);
                        if st.failures.get(&key).map(|(k, _)| *k) != Some(id) {
                            requests += 1;
                            let step = api.jobs(&w.repo, run.id).await.ok().and_then(|j: Vec<Job>| failed_step(&j));
                            st.failures.insert(key.clone(), (id, step));
                        }
                        card.failed_step = st.failures.get(&key).and_then(|(_, s)| s.clone());
                    }
                }
                let (seen, kind) = decide(st.seen.get(&key), &run, st.first_poll, &w.notify);
                st.seen.insert(key.clone(), seen);
                if let Some(kind) = kind {
                    notifications.push(Notification {
                        watched: w.clone(),
                        run: run.clone(),
                        kind,
                        failed_step: card.failed_step.clone(),
                    });
                }
            }
        }
        match groups.iter_mut().find(|g| g.repo == w.repo) {
            Some(g) => g.cards.push(card),
            None => groups.push(RepoGroup { repo: w.repo.clone(), cards: vec![card] }),
        }
    }
    st.first_poll = false;

    CycleOutput {
        groups,
        notifications,
        any_running,
        min_running_progress: running_progress.into_iter().reduce(f32::min),
        auth_error,
        requests,
    }
}

pub fn fmt_duration(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m {}s", s / 60, s % 60),
        s => format!("{}h {}m", s / 3600, (s % 3600) / 60),
    }
}

pub fn notification_text(
    w: &WatchedWorkflow,
    run: &RunSnapshot,
    kind: NotifyKind,
    failed_step: Option<&str>,
) -> (String, String) {
    let icon = match kind {
        NotifyKind::Succeeded => "✅",
        NotifyKind::Failed => "❌",
        NotifyKind::Started => "🟡",
    };
    let title = format!("{icon} {} · {}", w.workflow_name, w.branch.as_deref().unwrap_or("cualquiera"));
    let start = run.run_started_at.unwrap_or(run.created_at);
    let duration = fmt_duration((run.updated_at - start).num_seconds().max(0) as u64);
    let body = match (kind, failed_step) {
        (NotifyKind::Started, _) => format!("{} · empezó", w.repo),
        (NotifyKind::Failed, Some(step)) => format!("{} · falló en {step}", w.repo),
        _ => format!("{} · {duration}", w.repo),
    };
    (title, body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{Repo, Step, Workflow};
    use crate::model::NotifyPrefs;
    use std::collections::HashMap;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Mutex;

    #[derive(Default)]
    struct FakeApi {
        runs: Mutex<HashMap<String, Result<Option<RunSnapshot>, ApiError>>>,
        jobs: Mutex<HashMap<u64, Vec<Job>>>,
        durations: Vec<u64>,
        duration_calls: AtomicU32,
    }

    impl FakeApi {
        fn set_run(&self, w: &WatchedWorkflow, r: Result<Option<RunSnapshot>, ApiError>) {
            self.runs.lock().unwrap().insert(w.key(), r);
        }
    }

    impl GithubApi for FakeApi {
        async fn me(&self) -> Result<String, ApiError> {
            Ok("me".into())
        }
        async fn list_repos(&self) -> Result<Vec<Repo>, ApiError> {
            Ok(vec![])
        }
        async fn get_repo(&self, _: &str) -> Result<Repo, ApiError> {
            Err(ApiError::NotFound)
        }
        async fn list_workflows(&self, _: &str) -> Result<Vec<Workflow>, ApiError> {
            Ok(vec![])
        }
        async fn latest_run(&self, w: &WatchedWorkflow) -> Result<Option<RunSnapshot>, ApiError> {
            self.runs.lock().unwrap().get(&w.key()).cloned().unwrap_or(Ok(None))
        }
        async fn recent_success_durations(&self, _: &WatchedWorkflow) -> Result<Vec<u64>, ApiError> {
            self.duration_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.durations.clone())
        }
        async fn jobs(&self, _: &str, run_id: u64) -> Result<Vec<Job>, ApiError> {
            Ok(self.jobs.lock().unwrap().get(&run_id).cloned().unwrap_or_default())
        }
        fn rate(&self) -> RateInfo {
            RateInfo::default()
        }
    }

    fn wf(repo: &str, id: u64) -> WatchedWorkflow {
        WatchedWorkflow {
            repo: repo.into(),
            workflow_id: id,
            workflow_name: format!("WF{id}"),
            branch: Some("release".into()),
            event: Some("push".into()),
            notify: NotifyPrefs::default(),
        }
    }

    fn run(id: u64, status: &str, conclusion: Option<&str>, now: DateTime<Utc>) -> RunSnapshot {
        RunSnapshot {
            id,
            attempt: 1,
            status: status.into(),
            conclusion: conclusion.map(Into::into),
            html_url: format!("https://github.com/x/actions/runs/{id}"),
            created_at: now - chrono::Duration::seconds(120),
            run_started_at: Some(now - chrono::Duration::seconds(100)),
            updated_at: now,
        }
    }

    fn cfg(w: Vec<WatchedWorkflow>) -> Config {
        Config { watched: w, autostart: true }
    }

    #[tokio::test]
    async fn first_cycle_never_notifies() {
        let now = Utc::now();
        let api = FakeApi::default();
        let w = wf("acme/api", 1);
        api.set_run(&w, Ok(Some(run(5, "completed", Some("failure"), now))));
        let mut st = PollerState::new();
        let out = run_cycle(&api, &cfg(vec![w]), &mut st, now).await;
        assert!(out.notifications.is_empty());
        assert_eq!(out.groups[0].cards[0].light, Light::Red);
    }

    #[tokio::test]
    async fn running_then_success_notifies_once() {
        let now = Utc::now();
        let api = FakeApi { durations: vec![200], ..Default::default() };
        let w = wf("acme/api", 1);
        let c = cfg(vec![w.clone()]);
        let mut st = PollerState::new();

        api.set_run(&w, Ok(Some(run(5, "in_progress", None, now))));
        let out = run_cycle(&api, &c, &mut st, now).await;
        assert!(out.any_running);
        assert_eq!(out.min_running_progress, Some(0.5)); // 100s of a 200s median
        assert_eq!(out.groups[0].cards[0].expected_secs, Some(200));

        api.set_run(&w, Ok(Some(run(5, "completed", Some("success"), now))));
        let out = run_cycle(&api, &c, &mut st, now).await;
        assert_eq!(out.notifications.len(), 1);
        assert_eq!(out.notifications[0].kind, NotifyKind::Succeeded);
        assert!(!out.any_running);

        let out = run_cycle(&api, &c, &mut st, now).await;
        assert!(out.notifications.is_empty());
    }

    #[tokio::test]
    async fn failure_includes_failed_step() {
        let now = Utc::now();
        let api = FakeApi::default();
        let w = wf("acme/api", 1);
        let c = cfg(vec![w.clone()]);
        let mut st = PollerState::new();
        api.set_run(&w, Ok(Some(run(5, "in_progress", None, now))));
        run_cycle(&api, &c, &mut st, now).await;

        api.jobs.lock().unwrap().insert(
            5,
            vec![Job {
                status: "completed".into(),
                conclusion: Some("failure".into()),
                steps: vec![Step {
                    name: "Run migrations".into(),
                    status: "completed".into(),
                    conclusion: Some("failure".into()),
                    number: 1,
                }],
            }],
        );
        api.set_run(&w, Ok(Some(run(5, "completed", Some("failure"), now))));
        let out = run_cycle(&api, &c, &mut st, now).await;
        assert_eq!(out.notifications[0].kind, NotifyKind::Failed);
        assert_eq!(out.notifications[0].failed_step.as_deref(), Some("Run migrations"));
        assert_eq!(out.groups[0].cards[0].failed_step.as_deref(), Some("Run migrations"));
    }

    #[tokio::test]
    async fn median_fetched_once_per_run() {
        let now = Utc::now();
        let api = FakeApi { durations: vec![300], ..Default::default() };
        let w = wf("acme/api", 1);
        let c = cfg(vec![w.clone()]);
        let mut st = PollerState::new();
        api.set_run(&w, Ok(Some(run(5, "in_progress", None, now))));
        run_cycle(&api, &c, &mut st, now).await;
        run_cycle(&api, &c, &mut st, now).await;
        assert_eq!(api.duration_calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn one_workflow_error_does_not_block_others() {
        let now = Utc::now();
        let api = FakeApi::default();
        let a = wf("acme/api", 1);
        let b = wf("acme/api", 2);
        api.set_run(&a, Err(ApiError::NotFound));
        api.set_run(&b, Ok(Some(run(7, "completed", Some("success"), now))));
        let mut st = PollerState::new();
        let out = run_cycle(&api, &cfg(vec![a, b]), &mut st, now).await;
        let cards = &out.groups[0].cards;
        assert!(cards[0].error.is_some());
        assert_eq!(cards[0].light, Light::Gray);
        assert_eq!(cards[1].light, Light::Green);
        assert_eq!(out.auth_error, None);
    }

    #[tokio::test]
    async fn unauthorized_surfaces_auth_error() {
        let now = Utc::now();
        let api = FakeApi::default();
        let a = wf("acme/api", 1);
        api.set_run(&a, Err(ApiError::Unauthorized));
        let out = run_cycle(&api, &cfg(vec![a]), &mut PollerState::new(), now).await;
        assert_eq!(out.auth_error, Some(ApiError::Unauthorized));
    }

    #[tokio::test]
    async fn sso_surfaces_auth_error_with_url() {
        let now = Utc::now();
        let api = FakeApi::default();
        let a = wf("acme/api", 1);
        let sso = ApiError::Sso { url: Some("https://github.com/orgs/acme/sso".into()) };
        api.set_run(&a, Err(sso.clone()));
        let out = run_cycle(&api, &cfg(vec![a]), &mut PollerState::new(), now).await;
        assert_eq!(out.auth_error, Some(sso));
    }

    #[tokio::test]
    async fn groups_by_repo_in_config_order() {
        let now = Utc::now();
        let api = FakeApi::default();
        let c = cfg(vec![wf("acme/web", 1), wf("acme/api", 2), wf("acme/web", 3)]);
        let out = run_cycle(&api, &c, &mut PollerState::new(), now).await;
        let repos: Vec<_> = out.groups.iter().map(|g| (g.repo.as_str(), g.cards.len())).collect();
        assert_eq!(repos, vec![("acme/web", 2), ("acme/api", 1)]);
    }

    #[test]
    fn notification_text_variants() {
        let now = Utc::now();
        let w = wf("acme/api", 1);
        let mut r = run(5, "completed", Some("success"), now);
        r.run_started_at = Some(now - chrono::Duration::seconds(291));
        assert_eq!(
            notification_text(&w, &r, NotifyKind::Succeeded, None),
            ("✅ WF1 · release".to_string(), "acme/api · 4m 51s".to_string())
        );
        assert_eq!(
            notification_text(&w, &r, NotifyKind::Failed, Some("Run migrations")).1,
            "acme/api · falló en Run migrations"
        );
        let any = WatchedWorkflow { branch: None, ..w.clone() };
        assert_eq!(notification_text(&any, &r, NotifyKind::Started, None).0, "🟡 WF1 · cualquiera");
        assert_eq!(notification_text(&any, &r, NotifyKind::Started, None).1, "acme/api · empezó");
    }
}
