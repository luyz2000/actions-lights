use crate::model::Light;

const RUNNING: [&str; 5] = ["queued", "requested", "pending", "waiting", "in_progress"];

pub fn is_running(status: &str) -> bool {
    RUNNING.contains(&status)
}

pub fn light_for(status: &str, conclusion: Option<&str>) -> Light {
    if is_running(status) {
        return Light::Yellow;
    }
    match (status, conclusion) {
        ("completed", Some("success")) => Light::Green,
        ("completed", Some("failure" | "timed_out" | "startup_failure")) => Light::Red,
        _ => Light::Gray,
    }
}

pub fn label_for(status: &str) -> Option<String> {
    (status == "waiting").then(|| "esperando aprobación".to_string())
}

fn rank(l: Light) -> u8 {
    match l {
        Light::Gray => 0,
        Light::Green => 1,
        Light::Yellow => 2,
        Light::Red => 3,
    }
}

/// Worst state wins: Red > Yellow > Green > Gray.
pub fn aggregate(lights: &[Light]) -> Light {
    lights
        .iter()
        .copied()
        .max_by_key(|l| rank(*l))
        .unwrap_or(Light::Gray)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{NotifyPrefs, WatchedWorkflow};

    #[test]
    fn running_statuses_are_yellow() {
        for s in ["queued", "requested", "pending", "waiting", "in_progress"] {
            assert_eq!(light_for(s, None), Light::Yellow);
            assert!(is_running(s));
        }
    }

    #[test]
    fn success_is_green() {
        assert_eq!(light_for("completed", Some("success")), Light::Green);
        assert!(!is_running("completed"));
    }

    #[test]
    fn failures_are_red() {
        for c in ["failure", "timed_out", "startup_failure"] {
            assert_eq!(light_for("completed", Some(c)), Light::Red);
        }
    }

    #[test]
    fn non_code_outcomes_are_gray() {
        for c in [
            "cancelled",
            "skipped",
            "neutral",
            "stale",
            "action_required",
        ] {
            assert_eq!(light_for("completed", Some(c)), Light::Gray);
        }
    }

    #[test]
    fn unknown_status_is_gray() {
        assert_eq!(light_for("weird", None), Light::Gray);
    }

    #[test]
    fn waiting_has_label() {
        assert_eq!(
            label_for("waiting").as_deref(),
            Some("esperando aprobación")
        );
        assert_eq!(label_for("in_progress"), None);
    }

    #[test]
    fn aggregate_priority() {
        use Light::*;
        assert_eq!(aggregate(&[]), Gray);
        assert_eq!(aggregate(&[Green, Gray]), Green);
        assert_eq!(aggregate(&[Green, Yellow]), Yellow);
        assert_eq!(aggregate(&[Yellow, Red, Green]), Red);
    }

    #[test]
    fn watched_key_uses_star_for_any() {
        let w = WatchedWorkflow {
            repo: "o/r".into(),
            workflow_id: 7,
            workflow_name: "Deploy".into(),
            branch: None,
            event: Some("push".into()),
            notify: NotifyPrefs::default(),
        };
        assert_eq!(w.key(), "o/r#7#*#push");
    }

    #[test]
    fn notify_defaults() {
        let p = NotifyPrefs::default();
        assert!(p.on_success && p.on_failure && !p.on_start);
    }
}
