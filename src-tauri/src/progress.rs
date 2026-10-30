const CAP: f32 = 0.95;

#[derive(Clone, Debug, PartialEq)]
pub struct StepCount {
    pub done: u32,
    pub total: u32,
    pub current: Option<String>,
}

pub fn median_secs(durations: &[u64]) -> Option<u64> {
    if durations.is_empty() {
        return None;
    }
    let mut v = durations.to_vec();
    v.sort_unstable();
    let mid = v.len() / 2;
    Some(if v.len().is_multiple_of(2) {
        (v[mid - 1] + v[mid]) / 2
    } else {
        v[mid]
    })
}

/// Time against the typical duration when history exists (smoother), else step fraction.
/// Capped below 100% until the run actually completes.
pub fn estimate(elapsed_secs: u64, median: Option<u64>, steps: Option<&StepCount>) -> Option<f32> {
    let raw = match (median, steps) {
        (Some(m), _) if m > 0 => elapsed_secs as f32 / m as f32,
        (_, Some(s)) if s.total > 0 => s.done as f32 / s.total as f32,
        _ => return None,
    };
    Some(raw.min(CAP))
}

pub fn step_label(steps: &StepCount) -> Option<String> {
    if steps.total == 0 {
        return None;
    }
    let n = (steps.done + 1).min(steps.total);
    Some(match &steps.current {
        Some(name) => format!("Paso {n}/{} · {name}", steps.total),
        None => format!("Paso {n}/{}", steps.total),
    })
}

pub fn step_short(steps: &StepCount) -> Option<String> {
    (steps.total > 0).then(|| format!("{}/{}", (steps.done + 1).min(steps.total), steps.total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn median_odd_even() {
        assert_eq!(median_secs(&[]), None);
        assert_eq!(median_secs(&[300, 100, 200]), Some(200));
        assert_eq!(median_secs(&[100, 200, 300, 400]), Some(250));
    }

    #[test]
    fn time_based_capped() {
        assert_eq!(estimate(150, Some(300), None), Some(0.5));
        assert_eq!(estimate(900, Some(300), None), Some(0.95));
    }

    #[test]
    fn steps_fallback() {
        let s = StepCount {
            done: 4,
            total: 8,
            current: Some("Build image".into()),
        };
        assert_eq!(estimate(10, None, Some(&s)), Some(0.5));
        assert_eq!(step_label(&s).as_deref(), Some("Paso 5/8 · Build image"));
    }

    #[test]
    fn steps_capped_and_label_without_current() {
        let s = StepCount {
            done: 8,
            total: 8,
            current: None,
        };
        assert_eq!(estimate(10, None, Some(&s)), Some(0.95));
        assert_eq!(step_label(&s).as_deref(), Some("Paso 8/8"));
    }

    #[test]
    fn step_short_is_position_only() {
        let s = StepCount {
            done: 2,
            total: 5,
            current: Some("Build".into()),
        };
        assert_eq!(step_short(&s).as_deref(), Some("3/5"));
        let none = StepCount {
            done: 0,
            total: 0,
            current: None,
        };
        assert_eq!(step_short(&none), None);
    }

    #[test]
    fn zero_total_steps_is_unknown() {
        let s = StepCount {
            done: 0,
            total: 0,
            current: None,
        };
        assert_eq!(estimate(10, None, Some(&s)), None);
        assert_eq!(step_label(&s), None);
    }

    #[test]
    fn time_wins_over_steps() {
        let s = StepCount {
            done: 1,
            total: 10,
            current: None,
        };
        assert_eq!(estimate(150, Some(300), Some(&s)), Some(0.5));
    }

    #[test]
    fn nothing_known() {
        assert_eq!(estimate(10, None, None), None);
    }
}
