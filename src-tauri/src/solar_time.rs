use chrono::{DateTime, Datelike, Timelike, Utc};

pub fn inferred_timezone(latitude: Option<f64>, longitude: Option<f64>) -> (&'static str, &'static str) {
    match (latitude, longitude) {
        (Some(lat), Some(lon)) if (35.0..=43.0).contains(&lat) && (25.0..=45.0).contains(&lon) => ("Europe/Istanbul", "coordinates"),
        _ => ("Europe/Istanbul", "turkey_default"),
    }
}

pub fn solar_elevation_degrees(now: DateTime<Utc>, latitude: f64, longitude: f64) -> f64 {
    let day = now.ordinal() as f64;
    let hour = now.hour() as f64 + now.minute() as f64 / 60.0 + now.second() as f64 / 3600.0;
    let gamma = 2.0 * std::f64::consts::PI / 365.0 * (day - 1.0 + (hour - 12.0) / 24.0);
    let declination = 0.006918 - 0.399912 * gamma.cos() + 0.070257 * gamma.sin() - 0.006758 * (2.0 * gamma).cos() + 0.000907 * (2.0 * gamma).sin() - 0.002697 * (3.0 * gamma).cos() + 0.00148 * (3.0 * gamma).sin();
    let equation = 229.18 * (0.000075 + 0.001868 * gamma.cos() - 0.032077 * gamma.sin() - 0.014615 * (2.0 * gamma).cos() - 0.040849 * (2.0 * gamma).sin());
    let minutes = hour * 60.0;
    let true_solar_minutes = (minutes + equation + 4.0 * longitude).rem_euclid(1440.0);
    let hour_angle = (true_solar_minutes / 4.0 - 180.0).to_radians();
    let latitude = latitude.to_radians();
    (latitude.sin() * declination.sin() + latitude.cos() * declination.cos() * hour_angle.cos()).asin().to_degrees()
}

pub fn is_valid_daylight(now: DateTime<Utc>, latitude: Option<f64>, longitude: Option<f64>, peers_active: bool) -> bool {
    match (latitude, longitude) {
        (Some(lat), Some(lon)) => solar_elevation_degrees(now, lat, lon) > 5.0 && peers_active,
        _ => peers_active,
    }
}

#[cfg(test)]
mod tests {
    use chrono::TimeZone;
    use super::*;
    #[test] fn turkey_is_inferred_from_coordinates() { assert_eq!(inferred_timezone(Some(38.4), Some(27.1)).0, "Europe/Istanbul"); }
    #[test] fn izmir_summer_midday_is_daylight() {
        let midday = Utc.with_ymd_and_hms(2026, 7, 1, 9, 0, 0).unwrap();
        assert!(solar_elevation_degrees(midday, 38.4, 27.1) > 50.0);
    }
    #[test] fn izmir_midnight_is_not_daylight() {
        let midnight = Utc.with_ymd_and_hms(2026, 7, 1, 21, 0, 0).unwrap();
        assert!(solar_elevation_degrees(midnight, 38.4, 27.1) < 5.0);
    }
}
