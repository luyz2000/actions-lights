use crate::model::{Card, Light};
use chrono::{DateTime, Utc};

/// How long a finished run keeps its ✓/✗ next to the light.
const RECENT_SECS: i64 = 600;
const MAX_NAME: usize = 18;

fn short_name(name: &str) -> String {
    if name.chars().count() <= MAX_NAME {
        return name.to_string();
    }
    let cut: String = name.chars().take(MAX_NAME - 1).collect();
    format!("{}…", cut.trim_end())
}

fn short_duration(secs: u64) -> String {
    match secs {
        s if s < 60 => format!("{s}s"),
        s if s < 3600 => format!("{}m", s / 60),
        s => format!("{}h {}m", s / 3600, (s % 3600) / 60),
    }
}

fn elapsed(c: &Card, now: DateTime<Utc>) -> String {
    let secs = c
        .started_at
        .map(|s| (now - s).num_seconds().max(0) as u64)
        .unwrap_or(0);
    short_duration(secs)
}

/// Text shown next to the tray light, or None for the bare light.
pub fn tray_title(cards: &[Card], now: DateTime<Utc>) -> Option<String> {
    let running: Vec<&Card> = cards.iter().filter(|c| c.light == Light::Yellow).collect();
    match running.as_slice() {
        [] => {}
        [c] => {
            let mut parts = vec![short_name(&c.workflow_name)];
            if c.label.is_some() {
                parts.push("esperando".into());
            } else {
                parts.extend(c.step_short.clone());
                parts.push(elapsed(c, now));
            }
            return Some(parts.join(" · "));
        }
        many => {
            let slowest = many
                .iter()
                .min_by(|a, b| {
                    a.progress
                        .unwrap_or(0.0)
                        .total_cmp(&b.progress.unwrap_or(0.0))
                })
                .unwrap();
            return Some(format!(
                "{} en curso · {}",
                many.len(),
                elapsed(slowest, now)
            ));
        }
    }

    let latest = |light: Light| {
        cards
            .iter()
            .filter(|c| c.light == light)
            .filter_map(|c| c.completed_at.map(|t| (t, c)))
            .filter(|(t, _)| (now - *t).num_seconds() <= RECENT_SECS)
            .max_by_key(|(t, _)| *t)
            .map(|(_, c)| c)
    };
    if let Some(c) = latest(Light::Red) {
        return Some(format!("✗ {}", short_name(&c.workflow_name)));
    }
    latest(Light::Green).map(|c| format!("✓ {}", short_name(&c.workflow_name)))
}

/// One line per watched workflow, for the tray tooltip.
pub fn tray_tooltip(cards: &[Card]) -> String {
    if cards.is_empty() {
        return "Actions Lights · sin workflows".into();
    }
    let mut lines = vec!["Actions Lights".to_string()];
    for c in cards {
        let (mark, status) = match c.light {
            Light::Yellow => (
                "●",
                c.label
                    .clone()
                    .or_else(|| c.step_label.clone())
                    .unwrap_or_else(|| "en curso".into()),
            ),
            Light::Green => ("✓", "completado".into()),
            Light::Red => (
                "✗",
                c.failed_step
                    .as_ref()
                    .map(|s| format!("falló en {s}"))
                    .unwrap_or_else(|| "falló".into()),
            ),
            Light::Gray => (
                "○",
                c.error.clone().unwrap_or_else(|| "sin ejecuciones".into()),
            ),
        };
        lines.push(format!("{mark} {} ({}): {status}", c.workflow_name, c.repo));
    }
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    fn card(name: &str, light: Light) -> Card {
        Card {
            key: name.into(),
            repo: "acme/api".into(),
            workflow_name: name.into(),
            branch: None,
            light,
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

    fn running(name: &str, now: DateTime<Utc>, secs: i64, progress: f32) -> Card {
        Card {
            started_at: Some(now - Duration::seconds(secs)),
            progress: Some(progress),
            ..card(name, Light::Yellow)
        }
    }

    fn finished(name: &str, light: Light, now: DateTime<Utc>, ago: i64) -> Card {
        Card {
            completed_at: Some(now - Duration::seconds(ago)),
            ..card(name, light)
        }
    }

    #[test]
    fn idle_has_no_title() {
        let now = Utc::now();
        assert_eq!(tray_title(&[], now), None);
        assert_eq!(tray_title(&[card("deploy", Light::Gray)], now), None);
    }

    #[test]
    fn single_run_shows_name_step_and_elapsed() {
        let now = Utc::now();
        let c = Card {
            step_short: Some("3/5".into()),
            ..running("deploy", now, 130, 0.4)
        };
        assert_eq!(tray_title(&[c], now).as_deref(), Some("deploy · 3/5 · 2m"));
    }

    #[test]
    fn single_run_without_steps_omits_them() {
        let now = Utc::now();
        let c = running("deploy", now, 45, 0.1);
        assert_eq!(tray_title(&[c], now).as_deref(), Some("deploy · 45s"));
    }

    #[test]
    fn waiting_for_approval() {
        let now = Utc::now();
        let c = Card {
            label: Some("esperando aprobación".into()),
            ..running("deploy", now, 300, 0.0)
        };
        assert_eq!(tray_title(&[c], now).as_deref(), Some("deploy · esperando"));
    }

    #[test]
    fn several_runs_show_count_and_slowest_elapsed() {
        let now = Utc::now();
        let cards = [
            running("build", now, 600, 0.9),
            running("deploy", now, 200, 0.2),
            finished("lint", Light::Green, now, 30),
        ];
        assert_eq!(tray_title(&cards, now).as_deref(), Some("2 en curso · 3m"));
    }

    #[test]
    fn long_names_are_cut() {
        let now = Utc::now();
        let c = running("Build and publish docker image", now, 10, 0.1);
        assert_eq!(
            tray_title(&[c], now).as_deref(),
            Some("Build and publish… · 10s")
        );
    }

    #[test]
    fn recent_finish_shows_mark_for_ten_minutes() {
        let now = Utc::now();
        let ok = finished("deploy", Light::Green, now, 120);
        assert_eq!(tray_title(&[ok], now).as_deref(), Some("✓ deploy"));
        let old = finished("deploy", Light::Green, now, 601);
        assert_eq!(tray_title(&[old], now), None);
    }

    #[test]
    fn recent_failure_wins_over_recent_success() {
        let now = Utc::now();
        let cards = [
            finished("build", Light::Red, now, 300),
            finished("deploy", Light::Green, now, 10),
        ];
        assert_eq!(tray_title(&cards, now).as_deref(), Some("✗ build"));
    }

    #[test]
    fn running_wins_over_recent_finish() {
        let now = Utc::now();
        let cards = [
            finished("build", Light::Red, now, 30),
            running("deploy", now, 5, 0.0),
        ];
        assert_eq!(tray_title(&cards, now).as_deref(), Some("deploy · 5s"));
    }

    #[test]
    fn hour_long_runs() {
        let now = Utc::now();
        let c = running("deploy", now, 3900, 0.5);
        assert_eq!(tray_title(&[c], now).as_deref(), Some("deploy · 1h 5m"));
    }

    #[test]
    fn tooltip_lists_every_workflow() {
        let now = Utc::now();
        let cards = [
            Card {
                step_label: Some("Paso 3/5 · Build".into()),
                ..running("deploy", now, 10, 0.4)
            },
            Card {
                failed_step: Some("Run migrations".into()),
                ..finished("migrate", Light::Red, now, 10)
            },
            finished("lint", Light::Green, now, 10),
            card("docs", Light::Gray),
        ];
        assert_eq!(
            tray_tooltip(&cards),
            "Actions Lights\n\
             ● deploy (acme/api): Paso 3/5 · Build\n\
             ✗ migrate (acme/api): falló en Run migrations\n\
             ✓ lint (acme/api): completado\n\
             ○ docs (acme/api): sin ejecuciones"
        );
    }

    #[test]
    fn tooltip_without_workflows() {
        assert_eq!(tray_tooltip(&[]), "Actions Lights · sin workflows");
    }
}
