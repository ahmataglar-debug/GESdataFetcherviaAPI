use crate::domain::{Measurement, Severity, REQUIRED_BASELINE_DAYS};

#[derive(Debug, Clone)]
pub struct AnalysisConfig {
    pub schema_configured: bool,
    pub connected: bool,
    pub current_zero_threshold: f64,
    pub voltage_zero_threshold: f64,
    pub peers_active: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalysisOutcome {
    pub severity: Severity,
    pub reason: String,
}

pub fn analyze(
    measurement: &Measurement,
    valid_history: &[Measurement],
    peer_currents: &[f64],
    config: &AnalysisConfig,
) -> AnalysisOutcome {
    if !config.schema_configured {
        return outcome(Severity::Unconfigured, "GES bağlantı şeması hazırlanmalı");
    }
    if !config.connected {
        return outcome(Severity::Disconnected, "String şemada bağlantısız; alarm değerlendirmesine alınmadı");
    }
    let (Some(current), Some(voltage)) = (measurement.current, measurement.voltage) else {
        return outcome(Severity::NoData, "Akım veya gerilim değeri API cevabında yok");
    };
    if !measurement.is_valid_daylight {
        return outcome(Severity::Night, "Gece ölçümü analize dahil edilmedi");
    }
    if config.peers_active && current <= config.current_zero_threshold && voltage <= config.voltage_zero_threshold {
        return outcome(Severity::Critical, "Bağlı string gündüz üretim koşulunda sıfıra yakın");
    }

    let daily_history = distinct_daily_history(valid_history);
    if daily_history.len() < REQUIRED_BASELINE_DAYS {
        return outcome(
            Severity::Learning,
            &format!("30 geçerli gündüzün {} tanesi toplandı", daily_history.len()),
        );
    }

    let currents: Vec<f64> = daily_history.iter().filter_map(|item| item.current).collect();
    let voltages: Vec<f64> = daily_history.iter().filter_map(|item| item.voltage).collect();
    let current_score = robust_score(current, &currents);
    let voltage_score = robust_score(voltage, &voltages);
    let peer_median = median(peer_currents.to_vec());
    let peer_ratio = peer_median.filter(|value| *value > 0.01).map(|value| current / value);

    let historically_abnormal = current_score > 3.5 || voltage_score > 3.5;
    let peer_abnormal = peer_ratio.map(|ratio| !(0.55..=1.45).contains(&ratio)).unwrap_or(false);
    if historically_abnormal && peer_abnormal {
        return outcome(Severity::Suspicious, "Değer hem bir aylık geçmişten hem de kardeş stringlerden sapıyor");
    }
    outcome(Severity::Normal, "Değerler bir aylık karşılaştırma aralığında")
}

fn distinct_daily_history(history: &[Measurement]) -> Vec<&Measurement> {
    let mut sorted: Vec<_> = history.iter().filter(|item| item.is_valid_daylight && item.current.is_some() && item.voltage.is_some()).collect();
    sorted.sort_by_key(|item| item.sampled_at);
    sorted.dedup_by_key(|item| item.sampled_at.date_naive());
    sorted
}

fn robust_score(value: f64, samples: &[f64]) -> f64 {
    let Some(center) = median(samples.to_vec()) else { return 0.0 };
    let deviations: Vec<f64> = samples.iter().map(|sample| (sample - center).abs()).collect();
    let mad = median(deviations).unwrap_or_default();
    if mad < f64::EPSILON { if (value - center).abs() < f64::EPSILON { 0.0 } else { f64::INFINITY } } else { 0.6745 * (value - center).abs() / mad }
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() { return None; }
    values.sort_by(f64::total_cmp);
    let middle = values.len() / 2;
    Some(if values.len() % 2 == 0 { (values[middle - 1] + values[middle]) / 2.0 } else { values[middle] })
}

fn outcome(severity: Severity, reason: &str) -> AnalysisOutcome {
    AnalysisOutcome { severity, reason: reason.into() }
}

#[cfg(test)]
mod tests {
    use chrono::{Duration, TimeZone, Utc};
    use super::*;

    fn measurement(day: i64, current: Option<f64>, voltage: Option<f64>, daylight: bool) -> Measurement {
        Measurement { string_id: "s1".into(), current, voltage, sampled_at: Utc.with_ymd_and_hms(2026, 1, 1, 12, 0, 0).unwrap() + Duration::days(day), is_valid_daylight: daylight }
    }
    fn config() -> AnalysisConfig { AnalysisConfig { schema_configured: true, connected: true, current_zero_threshold: 0.15, voltage_zero_threshold: 10.0, peers_active: true } }

    #[test]
    fn waits_for_thirty_valid_days() {
        let history: Vec<_> = (0..29).map(|day| measurement(day, Some(9.0), Some(680.0), true)).collect();
        let result = analyze(&measurement(30, Some(9.1), Some(679.0), true), &history, &[9.0, 9.2], &config());
        assert_eq!(result.severity, Severity::Learning);
    }

    #[test]
    fn night_zero_is_not_critical() {
        let result = analyze(&measurement(0, Some(0.0), Some(0.0), false), &[], &[9.0], &config());
        assert_eq!(result.severity, Severity::Night);
    }

    #[test]
    fn connected_daylight_zero_is_critical() {
        let result = analyze(&measurement(0, Some(0.0), Some(0.0), true), &[], &[9.0], &config());
        assert_eq!(result.severity, Severity::Critical);
    }

    #[test]
    fn disconnected_string_is_neutral() {
        let mut disconnected = config();
        disconnected.connected = false;
        let result = analyze(&measurement(0, Some(0.0), Some(0.0), true), &[], &[9.0], &disconnected);
        assert_eq!(result.severity, Severity::Disconnected);
    }

    #[test]
    fn suspicious_requires_history_and_peer_confirmation() {
        let history: Vec<_> = (0..30).map(|day| measurement(day, Some(9.0 + (day % 3) as f64 * 0.1), Some(680.0 + (day % 2) as f64), true)).collect();
        let result = analyze(&measurement(31, Some(3.0), Some(610.0), true), &history, &[9.0, 9.2], &config());
        assert_eq!(result.severity, Severity::Suspicious);
    }
}
