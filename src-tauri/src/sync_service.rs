use std::collections::BTreeMap;

use chrono::{Duration, Timelike, Utc};
use chrono_tz::Tz;
use serde_json::Value;

use crate::{
    analysis::{analyze, AnalysisConfig},
    api::OpenApiClient,
    domain::{CloudStatus, DashboardSnapshot, MappedString, Measurement},
    error::{AppError, AppResult},
    secrets::SecretStore,
    scheduler::{decide, SyncDecision},
    solar_time::{inferred_timezone, is_valid_daylight},
    storage::Repository,
};

pub async fn synchronize(repository: &Repository, application_restarted: bool) -> AppResult<DashboardSnapshot> {
    let config = repository.api_configuration()?.ok_or_else(|| AppError::Configuration("OpenAPI yapılandırması henüz kaydedilmedi".into()))?;
    let client = OpenApiClient::from_secure_configuration(config)?;
    let plants = client.plants().await?;
    let plant_ids: Vec<_> = plants.iter().map(|plant| plant.id.clone()).collect();
    let details = client.plant_details(&plant_ids).await.unwrap_or_default();
    let now = Utc::now();
    let mut warnings = Vec::new();
    let mut inverter_ids = Vec::new();

    for listed in plants {
        let detail = details.iter().find(|item| item.id == listed.id);
        let latitude = detail.and_then(|item| item.latitude).or(listed.latitude);
        let longitude = detail.and_then(|item| item.longitude).or(listed.longitude);
        let (timezone, timezone_source) = inferred_timezone(latitude, longitude);
        let name = detail.filter(|item| !item.name.is_empty()).map(|item| item.name.as_str()).unwrap_or(&listed.name);
        let power_kw = detail.and_then(|item| item.power_kw).or(listed.power_kw);
        let cloud_status = detail.filter(|item| item.cloud_status != CloudStatus::Unknown).map(|item| &item.cloud_status).unwrap_or(&listed.cloud_status);
        let cloud_alarm_count = detail.map(|item| item.cloud_alarm_count).unwrap_or_default().max(listed.cloud_alarm_count);
        repository.upsert_plant(&listed.id, name, latitude, longitude, timezone, timezone_source, power_kw, cloud_status, cloud_alarm_count)?;
        for device in client.devices(&listed.id).await? {
            inverter_ids.push(device.id.clone());
            repository.upsert_inverter(
                &device.id,
                &listed.id,
                &device.name,
                &device.model,
                &device.serial_number,
                device.power_kw,
                &device.cloud_status,
                device.cloud_alarm_count,
                device.discovered_string_count,
            )?;
        }
    }

    match client.plant_realtime_power(&plant_ids).await {
        Ok(powers) => {
            for (plant_id, power_kw) in powers {
                repository.update_plant_power(&plant_id, power_kw)?;
            }
        }
        Err(error) => warnings.push(format!("Santral gerçek zamanlı güç verisi alınamadı: {error}")),
    }

    match client.inverter_realtime(&inverter_ids).await {
        Ok(inverters) => {
            for (inverter_id, realtime) in inverters {
                repository.update_inverter_realtime(&inverter_id, realtime.power_kw, realtime.strings.len())?;
                for string in realtime.strings {
                    let string_id = repository.upsert_discovered_string(
                        &inverter_id,
                        string.point.position,
                        string.point.current,
                        string.point.voltage,
                    )?;
                    repository.save_live_reading(&Measurement {
                        string_id,
                        current: string.current,
                        voltage: string.voltage,
                        sampled_at: now,
                        is_valid_daylight: false,
                    })?;
                }
            }
        }
        Err(_) => warnings.push("Santral gücü alınabiliyor ancak cihaz bazlı gerçek zamanlı API mevcut planda yanıt vermiyor. String akım/gerilim verileri için Device Real-time Data erişimi olan plan veya tarayıcı tabanlı veri kaynağı gerekir.".to_string()),
    }

    let (last_attempt, last_attempt_was_night) = repository.scheduler_state()?;
    let persist_analysis_sample = matches!(decide(now, last_attempt, last_attempt_was_night, application_restarted), SyncDecision::FetchNow);
    let mappings = repository.mapped_strings()?;
    let mut by_inverter: BTreeMap<String, Vec<MappedString>> = BTreeMap::new();
    for mapping in mappings {
        by_inverter.entry(mapping.inverter_id.clone()).or_default().push(mapping);
    }

    let mut daylight_flags = Vec::new();
    for (inverter_id, strings) in by_inverter {
        let mut point_ids: Vec<String> = strings.iter().flat_map(|item| [item.current_point_id.clone(), item.voltage_point_id.clone()]).collect();
        point_ids.sort();
        point_ids.dedup();
        let raw = match client.device_realtime_raw(std::slice::from_ref(&inverter_id), &point_ids).await {
            Ok(value) => value,
            Err(error) => {
                warnings.push(format!("{inverter_id}: string ölçümleri alınamadı ({error})"));
                continue;
            }
        };
        let Some(device_points) = first_device_points(&raw) else {
            warnings.push(format!("{inverter_id}: OpenAPI cevabında device_point bulunamadı"));
            continue;
        };
        let peer_currents: Vec<f64> = strings.iter().filter_map(|item| point_number(device_points, &item.current_point_id)).collect();
        let peers_active = peer_currents.iter().any(|value| *value > 0.5);

        for mapping in strings {
            let current = point_number(device_points, &mapping.current_point_id);
            let voltage = point_number(device_points, &mapping.voltage_point_id);
            let daylight = is_valid_daylight(now, mapping.latitude, mapping.longitude, peers_active);
            daylight_flags.push(daylight);
            let measurement = Measurement { string_id: mapping.id.clone(), current, voltage, sampled_at: now, is_valid_daylight: daylight };
            repository.save_live_reading(&measurement)?;
            if persist_analysis_sample {
                let history = repository.measurement_history(&mapping.id, now, 90)?;
                let history = same_local_time_history(history, &mapping.timezone, now, 90);
                let outcome = analyze(&measurement, &history, &peer_currents, &AnalysisConfig {
                    schema_configured: mapping.schema_configured,
                    connected: mapping.connected,
                    current_zero_threshold: mapping.current_zero_threshold,
                    voltage_zero_threshold: mapping.voltage_zero_threshold,
                    peers_active,
                });
                let measurement_id = repository.save_measurement(&measurement)?;
                repository.save_analysis(&mapping.id, measurement_id, &outcome.severity, &outcome.reason)?;
            }
        }
    }

    let all_mapped_samples_were_night = !daylight_flags.is_empty() && daylight_flags.iter().all(|value| !value);
    if persist_analysis_sample {
        let next = now + if all_mapped_samples_were_night { Duration::hours(8) } else { Duration::hours(24) };
        repository.mark_sync(now, next, all_mapped_samples_were_night)?;
    }
    let mut snapshot = repository.dashboard_snapshot(true, true)?;
    if !warnings.is_empty() {
        snapshot.warning = Some(warnings.join(" · "));
    }
    Ok(snapshot)
}

pub fn connection_is_ready(repository: &Repository) -> bool {
    repository.api_configuration().ok().flatten().is_some() && SecretStore::tokens().ok().flatten().is_some()
}

fn first_device_points(value: &Value) -> Option<&serde_json::Map<String, Value>> {
    let list = value.pointer("/result_data/device_point_list")?.as_array()?;
    let first = list.first()?;
    first.get("device_point").unwrap_or(first).as_object()
}

fn point_number(points: &serde_json::Map<String, Value>, point_id: &str) -> Option<f64> {
    let value = points.get(&format!("p{point_id}"))?;
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

fn same_local_time_history(history: Vec<Measurement>, timezone: &str, sample_time: chrono::DateTime<Utc>, tolerance_minutes: i64) -> Vec<Measurement> {
    let timezone = timezone.parse::<Tz>().unwrap_or(chrono_tz::Europe::Istanbul);
    let target = sample_time.with_timezone(&timezone);
    let target_minutes = target.hour() as i64 * 60 + target.minute() as i64;
    history.into_iter().filter(|item| {
        let local = item.sampled_at.with_timezone(&timezone);
        let minutes = local.hour() as i64 * 60 + local.minute() as i64;
        let difference = (minutes - target_minutes).abs();
        difference.min(1440 - difference) <= tolerance_minutes
    }).collect()
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use serde_json::json;
    use super::*;

    #[test]
    fn parses_legacy_device_point_shape() {
        let value = json!({"result_data":{"device_point_list":[{"device_point":{"p13001":"9.4","p13002":680.5}}]}});
        let points = first_device_points(&value).unwrap();
        assert_eq!(point_number(points, "13001"), Some(9.4));
        assert_eq!(point_number(points, "13002"), Some(680.5));
    }

    #[test]
    fn history_is_limited_to_same_local_time_window() {
        let sample = Utc.with_ymd_and_hms(2026, 7, 3, 9, 0, 0).unwrap();
        let history = vec![
            Measurement { string_id: "s".into(), current: Some(9.0), voltage: Some(680.0), sampled_at: sample - Duration::days(1), is_valid_daylight: true },
            Measurement { string_id: "s".into(), current: Some(8.0), voltage: Some(670.0), sampled_at: sample - Duration::days(1) - Duration::hours(4), is_valid_daylight: true },
        ];
        assert_eq!(same_local_time_history(history, "Europe/Istanbul", sample, 90).len(), 1);
    }
}
