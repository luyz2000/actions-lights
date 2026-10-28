use crate::model::{Light, NotifyPrefs, RunSnapshot};
use crate::status::{is_running, light_for};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NotifyKind {
    Started,
    Succeeded,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Seen {
    pub run_id: u64,
    pub attempt: u32,
    pub light: Light,
    pub running: bool,
}

/// Records what we observed and decides whether this observation is a transition worth
/// notifying. A workflow with no previous observation (app start, or newly watched) only
/// registers, so runs that finished before we were looking never notify.
pub fn decide(
    prev: Option<&Seen>,
    current: &RunSnapshot,
    first_poll: bool,
    prefs: &NotifyPrefs,
) -> (Seen, Option<NotifyKind>) {
    let running = is_running(&current.status);
    let seen = Seen {
        run_id: current.id,
        attempt: current.attempt,
        light: light_for(&current.status, current.conclusion.as_deref()),
        running,
    };
    let Some(prev) = prev.filter(|_| !first_poll) else {
        return (seen, None);
    };
    let same_run = prev.run_id == seen.run_id && prev.attempt == seen.attempt;

    let kind = if running {
        (!same_run || !prev.running).then_some(NotifyKind::Started)
    } else if !same_run || prev.running {
        match seen.light {
            Light::Green => Some(NotifyKind::Succeeded),
            Light::Red => Some(NotifyKind::Failed),
            _ => None,
        }
    } else {
        None
    };

    let allowed = match kind {
        Some(NotifyKind::Started) => prefs.on_start,
        Some(NotifyKind::Succeeded) => prefs.on_success,
        Some(NotifyKind::Failed) => prefs.on_failure,
        None => false,
    };
    (seen, kind.filter(|_| allowed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn snap(id: u64, attempt: u32, status: &str, conclusion: Option<&str>) -> RunSnapshot {
        let now = Utc::now();
        RunSnapshot {
            id,
            attempt,
            status: status.into(),
            conclusion: conclusion.map(Into::into),
            html_url: format!("https://github.com/o/r/actions/runs/{id}"),
            created_at: now,
            run_started_at: Some(now),
            updated_at: now,
        }
    }

    fn seen(run_id: u64, attempt: u32, light: Light, running: bool) -> Seen {
        Seen {
            run_id,
            attempt,
            light,
            running,
        }
    }

    fn prefs() -> NotifyPrefs {
        NotifyPrefs::default()
    }

    #[test]
    fn first_poll_is_silent() {
        let (s, n) = decide(
            None,
            &snap(1, 1, "completed", Some("success")),
            true,
            &prefs(),
        );
        assert_eq!(n, None);
        assert_eq!(s, seen(1, 1, Light::Green, false));
    }

    #[test]
    fn running_to_success_notifies() {
        let prev = seen(1, 1, Light::Yellow, true);
        let (_, n) = decide(
            Some(&prev),
            &snap(1, 1, "completed", Some("success")),
            false,
            &prefs(),
        );
        assert_eq!(n, Some(NotifyKind::Succeeded));
    }

    #[test]
    fn queued_to_failure_unseen_running_notifies() {
        let prev = seen(1, 1, Light::Green, false);
        let (_, n) = decide(
            Some(&prev),
            &snap(2, 1, "completed", Some("failure")),
            false,
            &prefs(),
        );
        assert_eq!(n, Some(NotifyKind::Failed));
    }

    #[test]
    fn newly_watched_completed_run_is_silent() {
        // Workflow added to config mid-session: first sighting already completed.
        let (_, n) = decide(
            None,
            &snap(5, 1, "completed", Some("success")),
            false,
            &prefs(),
        );
        assert_eq!(n, None);
    }

    #[test]
    fn same_completed_run_does_not_repeat() {
        let prev = seen(2, 1, Light::Red, false);
        let (_, n) = decide(
            Some(&prev),
            &snap(2, 1, "completed", Some("failure")),
            false,
            &prefs(),
        );
        assert_eq!(n, None);
    }

    #[test]
    fn rerun_failure_notifies_again() {
        let prev = seen(2, 1, Light::Red, false);
        let (s, n) = decide(
            Some(&prev),
            &snap(2, 2, "completed", Some("failure")),
            false,
            &prefs(),
        );
        assert_eq!(n, Some(NotifyKind::Failed));
        assert_eq!(s.attempt, 2);
    }

    #[test]
    fn cancelled_is_silent() {
        let prev = seen(3, 1, Light::Yellow, true);
        let (_, n) = decide(
            Some(&prev),
            &snap(3, 1, "completed", Some("cancelled")),
            false,
            &prefs(),
        );
        assert_eq!(n, None);
    }

    #[test]
    fn start_respects_prefs() {
        let prev = seen(1, 1, Light::Green, false);
        let current = snap(2, 1, "in_progress", None);
        assert_eq!(decide(Some(&prev), &current, false, &prefs()).1, None);
        let on = NotifyPrefs {
            on_start: true,
            ..prefs()
        };
        assert_eq!(
            decide(Some(&prev), &current, false, &on).1,
            Some(NotifyKind::Started)
        );
    }

    #[test]
    fn still_running_does_not_restart() {
        let on = NotifyPrefs {
            on_start: true,
            ..prefs()
        };
        let prev = seen(2, 1, Light::Yellow, true);
        assert_eq!(
            decide(Some(&prev), &snap(2, 1, "in_progress", None), false, &on).1,
            None
        );
    }

    #[test]
    fn success_respects_prefs() {
        let prev = seen(1, 1, Light::Yellow, true);
        let off = NotifyPrefs {
            on_success: false,
            ..prefs()
        };
        assert_eq!(
            decide(
                Some(&prev),
                &snap(1, 1, "completed", Some("success")),
                false,
                &off
            )
            .1,
            None
        );
    }

    #[test]
    fn failure_respects_prefs() {
        let prev = seen(1, 1, Light::Yellow, true);
        let off = NotifyPrefs {
            on_failure: false,
            ..prefs()
        };
        assert_eq!(
            decide(
                Some(&prev),
                &snap(1, 1, "completed", Some("failure")),
                false,
                &off
            )
            .1,
            None
        );
    }
}
