use crate::github::RateInfo;
use chrono::{DateTime, Utc};
use std::time::Duration;

const RUNNING: u64 = 15;
const IDLE: u64 = 60;
const NO_TOKEN: u64 = 300;

/// Base cadence, stretched so the remaining rate budget lasts until the window resets.
pub fn next_interval(
    any_running: bool,
    has_token: bool,
    rate: &RateInfo,
    now: DateTime<Utc>,
    requests_per_cycle: u32,
) -> Duration {
    let mut secs = if any_running { RUNNING } else { IDLE };
    if !has_token {
        secs = secs.max(NO_TOKEN);
    }
    if let (Some(remaining), Some(reset_at)) = (rate.remaining, rate.reset_at) {
        let window = (reset_at - now).num_seconds().max(0) as u64;
        let spread = window * requests_per_cycle as u64 / remaining.max(1) as u64;
        secs = secs.max(spread);
    }
    Duration::from_secs(secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none() -> RateInfo {
        RateInfo::default()
    }

    #[test]
    fn base_intervals() {
        let now = Utc::now();
        assert_eq!(
            next_interval(true, true, &none(), now, 3),
            Duration::from_secs(15)
        );
        assert_eq!(
            next_interval(false, true, &none(), now, 3),
            Duration::from_secs(60)
        );
        assert_eq!(
            next_interval(false, false, &none(), now, 3),
            Duration::from_secs(300)
        );
        assert_eq!(
            next_interval(true, false, &none(), now, 3),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn spreads_when_low_remaining() {
        let now = Utc::now();
        let rate = RateInfo {
            remaining: Some(10),
            reset_at: Some(now + chrono::Duration::seconds(600)),
        };
        assert_eq!(
            next_interval(true, true, &rate, now, 5),
            Duration::from_secs(300)
        );
    }

    #[test]
    fn plenty_of_budget_keeps_base() {
        let now = Utc::now();
        let rate = RateInfo {
            remaining: Some(4000),
            reset_at: Some(now + chrono::Duration::seconds(600)),
        };
        assert_eq!(
            next_interval(true, true, &rate, now, 5),
            Duration::from_secs(15)
        );
    }

    #[test]
    fn exhausted_waits_until_reset() {
        let now = Utc::now();
        let rate = RateInfo {
            remaining: Some(0),
            reset_at: Some(now + chrono::Duration::seconds(900)),
        };
        assert_eq!(
            next_interval(true, true, &rate, now, 1),
            Duration::from_secs(900)
        );
    }
}
