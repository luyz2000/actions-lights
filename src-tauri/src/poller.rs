use crate::github::{ApiError, GithubApi, Job, RateInfo, failed_step, steps_of};
use crate::model::{Card, Config, Light, RepoGroup, RunSnapshot, WatchedWorkflow};
use crate::notify_rules::{NotifyKind, Seen, decide};
use crate::progress::{estimate, median_secs, step_label, step_short};
use crate::status::{is_running, label_for, light_for};
use chrono::{DateTime, Utc};
use std::collections::HashMap;

#[derive(Clone, Debug, PartialEq)]
pub struct Notification {
    pub watched: WatchedWorkflow,
    pub run: RunSnapshot,
    pub kind: NotifyKind,
    pub failed_step: Option<String>,
    /// Typical duration from recent successful runs.
    pub expected_secs: Option<u64>,
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
        Self {
            seen: HashMap::new(),
            medians: HashMap::new(),
            failures: HashMap::new(),
            first_poll: true,
        }
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
        step_short: None,
        failed_step: None,
        error: None,
        run_branch: None,
        run_event: None,
    }
}

pub async fn run_cycle<A: GithubApi>(
    api: &A,
    cfg: &Config,
    st: &mut PollerState,
    now: DateTime<Utc>,
) -> CycleOutput {
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
                if matches!(e, ApiError::Unauthorized | ApiError::Sso { .. })
                    && auth_error.is_none()
                {
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
                card.run_branch = run.head_branch.clone();
                card.run_event = run.event.clone();
                card.started_at = run.run_started_at.or(Some(run.created_at));
                if running {
                    any_running = true;
                    if st.medians.get(&key).map(|(id, _)| *id) != Some(run.id) {
                        requests += 1;
                        let median = api
                            .recent_success_durations(w)
                            .await
                            .ok()
                            .and_then(|d| median_secs(&d));
                        st.medians.insert(key.clone(), (run.id, median));
                    }
                    let median = st.medians.get(&key).and_then(|(_, m)| *m);
                    requests += 1;
                    let steps = api.jobs(&w.repo, run.id).await.ok().map(|j| steps_of(&j));
                    let elapsed = card
                        .started_at
                        .map(|s| (now - s).num_seconds().max(0) as u64)
                        .unwrap_or(0);
                    card.progress = estimate(elapsed, median, steps.as_ref());
                    card.expected_secs = median;
                    card.step_label = steps.as_ref().and_then(step_label);
                    card.step_short = steps.as_ref().and_then(step_short);
                    running_progress.push(card.progress.unwrap_or(0.0));
                } else {
                    card.completed_at = Some(run.updated_at);
                    if card.light == Light::Red {
                        let id = (run.id, run.attempt);
                        if st.failures.get(&key).map(|(k, _)| *k) != Some(id) {
                            requests += 1;
                            let step = api
                                .jobs(&w.repo, run.id)
                                .await
                                .ok()
                                .and_then(|j: Vec<Job>| failed_step(&j));
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
                        expected_secs: card.expected_secs,
                    });
                }
            }
        }
        match groups.iter_mut().find(|g| g.repo == w.repo) {
            Some(g) => g.cards.push(card),
            None => groups.push(RepoGroup {
                repo: w.repo.clone(),
                cards: vec![card],
            }),
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
    expected_secs: Option<u64>,
) -> (String, String) {
    let title = match kind {
        NotifyKind::Succeeded => format!("✅ {} completado", w.workflow_name),
        NotifyKind::Failed => format!("❌ {} falló", w.workflow_name),
        NotifyKind::Started => format!("🟡 {} en curso", w.workflow_name),
    };
    let mut parts = Vec::new();
    if let (NotifyKind::Failed, Some(step)) = (kind, failed_step) {
        parts.push(format!("Falló en «{step}»"));
    }
    parts.push(w.repo.clone());
    parts.extend(run.head_branch.clone());
    parts.extend(run.event.clone());
    match kind {
        NotifyKind::Started => {
            if let Some(secs) = expected_secs {
                parts.push(format!("~{}m estimado", secs.div_ceil(60)));
            }
        }
        _ => {
            let start = run.run_started_at.unwrap_or(run.created_at);
            parts.push(fmt_duration(
                (run.updated_at - start).num_seconds().max(0) as u64
            ));
        }
    }
    (title, parts.join(" · "))
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Banner {
    pub text: Option<String>,
    /// Link the banner points to (SSO authorization page).
    pub url: Option<String>,
}

/// `stretched_to` is the poll interval when the rate budget forced it above its base.
fn banner_for(
    auth_error: Option<&ApiError>,
    has_token: bool,
    stretched_to: Option<std::time::Duration>,
) -> Banner {
    let text = |s: &str| Some(s.to_string());
    match auth_error {
        Some(ApiError::Sso { url }) => Banner {
            text: text("Autoriza el token para la organización (SSO) — clic para abrir"),
            url: url.clone(),
        },
        Some(ApiError::Unauthorized) => Banner {
            text: text(
                "Sesión de GitHub inválida: revisa `gh auth login` o el token en Repos y workflows…",
            ),
            url: None,
        },
        _ if !has_token => Banner {
            text: text("Sin token: solo repos públicos, actualizando cada 5 min"),
            url: None,
        },
        _ => match stretched_to {
            Some(d) => Banner {
                text: Some(format!(
                    "Límite de la API bajo: actualizando cada {} min",
                    d.as_secs().div_ceil(60)
                )),
                url: None,
            },
            None => Banner::default(),
        },
    }
}

/// Re-resolve the token (gh may have been logged in/refreshed since) while auth is broken
/// or missing. Called at most once per cycle, so it can't spin.
fn needs_auth_refresh(auth_error: Option<&ApiError>, has_token: bool) -> bool {
    matches!(auth_error, Some(ApiError::Unauthorized)) || !has_token
}

/// A whole cycle is bounded too, on top of per-request timeouts.
const CYCLE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

/// Background loop. Ticks every few seconds against the wall clock (not a monotonic sleep)
/// so a poll that came due while the Mac was asleep runs right after wake.
pub fn start(app: tauri::AppHandle) {
    use crate::app_state::AppCtx;
    use std::time::{Duration, SystemTime};
    use tauri::{Emitter, Manager};
    use tauri_plugin_notification::NotificationExt;

    tauri::async_runtime::spawn(async move {
        let ctx = app.state::<AppCtx>();
        let mut st = PollerState::new();
        let mut next_due = SystemTime::now();
        let mut retried_auth = false;
        loop {
            tokio::select! {
                _ = ctx.wake.notified() => {}
                _ = tokio::time::sleep(Duration::from_secs(5)) => {
                    if SystemTime::now() < next_due { continue; }
                }
            }
            let cfg = ctx.cfg.read().unwrap().clone();
            let now = Utc::now();
            let Ok(out) =
                tokio::time::timeout(CYCLE_TIMEOUT, run_cycle(&*ctx.api, &cfg, &mut st, now)).await
            else {
                next_due = SystemTime::now() + Duration::from_secs(15);
                continue;
            };

            // A stale gh token: re-resolve once and poll again right away.
            if out.auth_error == Some(ApiError::Unauthorized) && !retried_auth {
                retried_auth = true;
                ctx.refresh_auth().await;
                next_due = SystemTime::now();
                ctx.wake.notify_one();
                continue;
            }
            if out.auth_error.is_none() {
                retried_auth = false;
            }
            // Later recovery (e.g. `gh auth login` after a revoke): pick it up next cycle.
            if needs_auth_refresh(out.auth_error.as_ref(), ctx.api.has_token()) {
                ctx.refresh_auth().await;
            }

            let has_token = ctx.api.has_token();
            let lights: Vec<Light> = out
                .groups
                .iter()
                .flat_map(|g| g.cards.iter().map(|c| c.light))
                .collect();
            let overall = crate::status::aggregate(&lights);
            let ring =
                (overall == Light::Yellow).then_some(out.min_running_progress.unwrap_or(0.0));
            crate::tray::set_icon(&app, overall, ring);
            let cards: Vec<crate::model::Card> =
                out.groups.iter().flat_map(|g| g.cards.clone()).collect();
            crate::tray::set_text(
                &app,
                crate::tray_text::tray_title(&cards, now).as_deref(),
                &crate::tray_text::tray_tooltip(&cards),
            );

            let rate = ctx.api.rate();
            let wait = crate::interval::next_interval(
                out.any_running,
                has_token,
                &rate,
                now,
                out.requests.max(1),
            );
            let base = crate::interval::next_interval(
                out.any_running,
                has_token,
                &RateInfo::default(),
                now,
                1,
            );
            let banner = banner_for(
                out.auth_error.as_ref(),
                has_token,
                (wait > base).then_some(wait),
            );

            let dashboard = crate::model::DashboardState {
                groups: out.groups,
                auth: ctx.auth.read().unwrap().clone(),
                banner: banner.text,
                banner_url: banner.url,
                last_updated: Some(now),
            };
            *ctx.dashboard.write().unwrap() = dashboard.clone();
            let _ = app.emit("dashboard", &dashboard);

            for n in &out.notifications {
                let (title, body) = notification_text(
                    &n.watched,
                    &n.run,
                    n.kind,
                    n.failed_step.as_deref(),
                    n.expected_secs,
                );
                let mut builder = app.notification().builder().title(title).body(body);
                if let Some(sound) = crate::notify_rules::sound_for(n.kind, cfg.sound) {
                    builder = builder.sound(sound);
                }
                let _ = builder.show();
            }

            next_due = SystemTime::now() + wait;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::github::{Repo, Step, Workflow};
    use crate::model::NotifyPrefs;
    use std::collections::HashMap;
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicU32, Ordering};

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
            self.runs
                .lock()
                .unwrap()
                .get(&w.key())
                .cloned()
                .unwrap_or(Ok(None))
        }
        async fn recent_success_durations(
            &self,
            _: &WatchedWorkflow,
        ) -> Result<Vec<u64>, ApiError> {
            self.duration_calls.fetch_add(1, Ordering::SeqCst);
            Ok(self.durations.clone())
        }
        async fn jobs(&self, _: &str, run_id: u64) -> Result<Vec<Job>, ApiError> {
            Ok(self
                .jobs
                .lock()
                .unwrap()
                .get(&run_id)
                .cloned()
                .unwrap_or_default())
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
            head_branch: Some("release".into()),
            event: Some("push".into()),
        }
    }

    fn cfg(w: Vec<WatchedWorkflow>) -> Config {
        Config {
            watched: w,
            ..Config::default()
        }
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
        let api = FakeApi {
            durations: vec![200],
            ..Default::default()
        };
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
        assert_eq!(
            out.notifications[0].failed_step.as_deref(),
            Some("Run migrations")
        );
        assert_eq!(
            out.groups[0].cards[0].failed_step.as_deref(),
            Some("Run migrations")
        );
    }

    #[tokio::test]
    async fn median_fetched_once_per_run() {
        let now = Utc::now();
        let api = FakeApi {
            durations: vec![300],
            ..Default::default()
        };
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
        let sso = ApiError::Sso {
            url: Some("https://github.com/orgs/acme/sso".into()),
        };
        api.set_run(&a, Err(sso.clone()));
        let out = run_cycle(&api, &cfg(vec![a]), &mut PollerState::new(), now).await;
        assert_eq!(out.auth_error, Some(sso));
    }

    #[tokio::test]
    async fn groups_by_repo_in_config_order() {
        let now = Utc::now();
        let api = FakeApi::default();
        let c = cfg(vec![
            wf("acme/web", 1),
            wf("acme/api", 2),
            wf("acme/web", 3),
        ]);
        let out = run_cycle(&api, &c, &mut PollerState::new(), now).await;
        let repos: Vec<_> = out
            .groups
            .iter()
            .map(|g| (g.repo.as_str(), g.cards.len()))
            .collect();
        assert_eq!(repos, vec![("acme/web", 2), ("acme/api", 1)]);
    }

    #[test]
    fn sso_banner_carries_clickable_url() {
        let sso = ApiError::Sso {
            url: Some("https://github.com/orgs/acme/sso?x=1".into()),
        };
        let b = banner_for(Some(&sso), true, None);
        assert_eq!(
            b.url.as_deref(),
            Some("https://github.com/orgs/acme/sso?x=1")
        );
        assert!(b.text.unwrap().contains("Autoriza el token"));
    }

    #[test]
    fn stretched_interval_explains_rate_limit() {
        let b = banner_for(None, true, Some(std::time::Duration::from_secs(300)));
        assert_eq!(
            b.text.as_deref(),
            Some("Límite de la API bajo: actualizando cada 5 min")
        );
        assert_eq!(b.url, None);
    }

    #[test]
    fn healthy_has_no_banner() {
        assert_eq!(banner_for(None, true, None), Banner::default());
    }

    #[test]
    fn auth_refresh_while_unauthorized_or_tokenless() {
        assert!(needs_auth_refresh(Some(&ApiError::Unauthorized), true));
        assert!(needs_auth_refresh(None, false));
        assert!(!needs_auth_refresh(None, true));
        assert!(!needs_auth_refresh(Some(&ApiError::NotFound), true));
    }

    #[test]
    fn notification_text_variants() {
        let now = Utc::now();
        let w = wf("acme/api", 1);
        let mut r = run(5, "completed", Some("success"), now);
        r.run_started_at = Some(now - chrono::Duration::seconds(291));
        assert_eq!(
            notification_text(&w, &r, NotifyKind::Succeeded, None, None),
            (
                "✅ WF1 completado".to_string(),
                "acme/api · release · push · 4m 51s".to_string()
            )
        );
        assert_eq!(
            notification_text(&w, &r, NotifyKind::Failed, Some("Run migrations"), None),
            (
                "❌ WF1 falló".to_string(),
                "Falló en «Run migrations» · acme/api · release · push · 4m 51s".to_string()
            )
        );
        assert_eq!(
            notification_text(&w, &r, NotifyKind::Failed, None, None).1,
            "acme/api · release · push · 4m 51s"
        );
        let any = WatchedWorkflow {
            branch: None,
            ..w.clone()
        };
        assert_eq!(
            notification_text(&any, &r, NotifyKind::Started, None, Some(300)),
            (
                "🟡 WF1 en curso".to_string(),
                "acme/api · release · push · ~5m estimado".to_string()
            )
        );
        assert_eq!(
            notification_text(&any, &r, NotifyKind::Started, None, None).1,
            "acme/api · release · push"
        );
    }

    #[test]
    fn notification_shows_the_run_branch_not_the_filter() {
        let now = Utc::now();
        let w = WatchedWorkflow {
            branch: None,
            event: None,
            ..wf("acme/api", 1)
        };
        let r = RunSnapshot {
            head_branch: Some("hotfix/login".into()),
            event: Some("workflow_dispatch".into()),
            ..run(5, "completed", Some("success"), now)
        };
        assert_eq!(
            notification_text(&w, &r, NotifyKind::Succeeded, None, None).1,
            "acme/api · hotfix/login · workflow_dispatch · 1m 40s"
        );
        let bare = RunSnapshot {
            head_branch: None,
            event: None,
            ..r
        };
        assert_eq!(
            notification_text(&w, &bare, NotifyKind::Succeeded, None, None).1,
            "acme/api · 1m 40s"
        );
    }

    #[tokio::test]
    async fn card_carries_the_run_branch_and_event() {
        let now = Utc::now();
        let api = FakeApi::default();
        let w = wf("acme/api", 1);
        api.set_run(&w, Ok(Some(run(5, "completed", Some("success"), now))));
        let out = run_cycle(&api, &cfg(vec![w]), &mut PollerState::new(), now).await;
        let card = &out.groups[0].cards[0];
        assert_eq!(
            (card.run_branch.as_deref(), card.run_event.as_deref()),
            (Some("release"), Some("push"))
        );
    }

    #[test]
    fn started_estimate_rounds_up_to_a_minute() {
        let now = Utc::now();
        let w = WatchedWorkflow {
            branch: None,
            ..wf("acme/api", 1)
        };
        let r = run(5, "in_progress", None, now);
        assert_eq!(
            notification_text(&w, &r, NotifyKind::Started, None, Some(20)).1,
            "acme/api · release · push · ~1m estimado"
        );
    }

    #[tokio::test]
    async fn notification_carries_expected_duration() {
        let now = Utc::now();
        let api = FakeApi {
            durations: vec![240, 300, 360],
            ..FakeApi::default()
        };
        let w = WatchedWorkflow {
            notify: NotifyPrefs {
                on_start: true,
                ..NotifyPrefs::default()
            },
            ..wf("acme/api", 1)
        };
        let c = cfg(vec![w.clone()]);
        let mut st = PollerState::new();
        api.set_run(&w, Ok(Some(run(4, "completed", Some("success"), now))));
        run_cycle(&api, &c, &mut st, now).await;
        api.set_run(&w, Ok(Some(run(5, "in_progress", None, now))));
        let out = run_cycle(&api, &c, &mut st, now).await;
        assert_eq!(out.notifications.len(), 1);
        assert_eq!(out.notifications[0].expected_secs, Some(300));
    }
}
