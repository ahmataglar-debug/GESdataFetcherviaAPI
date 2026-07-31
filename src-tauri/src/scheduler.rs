use chrono::{DateTime, Duration, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncDecision { FetchNow, WaitUntil(DateTime<Utc>) }

pub fn decide(now: DateTime<Utc>, last_attempt: Option<DateTime<Utc>>, last_attempt_was_night: bool, application_restarted: bool) -> SyncDecision {
    let Some(last) = last_attempt else { return SyncDecision::FetchNow };
    let cooldown = if last_attempt_was_night { Duration::hours(8) } else if application_restarted { Duration::hours(12) } else { Duration::hours(24) };
    let due = last + cooldown;
    if now >= due { SyncDecision::FetchNow } else { SyncDecision::WaitUntil(due) }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use super::*;
    #[test] fn night_retry_uses_eight_hours() {
        let last = Utc.with_ymd_and_hms(2026, 7, 1, 22, 0, 0).unwrap();
        assert_eq!(decide(last + Duration::hours(8), Some(last), true, false), SyncDecision::FetchNow);
    }
    #[test] fn restart_catchup_uses_twelve_hours() {
        let last = Utc.with_ymd_and_hms(2026, 7, 1, 12, 0, 0).unwrap();
        assert_eq!(decide(last + Duration::hours(12), Some(last), false, true), SyncDecision::FetchNow);
    }
}

