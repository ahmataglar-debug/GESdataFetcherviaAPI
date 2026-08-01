use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};
use std::collections::BTreeMap;

use super::points::{inverter_point_batches, StringPoint, INVERTER_ACTIVE_POWER, PLANT_ACTIVE_POWER, STRING_POINTS};

use crate::{
    domain::{CloudStatus, StoredApiConfiguration},
    error::{AppError, AppResult},
    secrets::{OAuthTokens, SecretStore},
};

#[derive(Debug, Clone)]
pub struct RemotePlant {
    pub id: String,
    pub name: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub power_kw: Option<f64>,
    pub cloud_status: CloudStatus,
    pub cloud_alarm_count: usize,
}

#[derive(Debug, Clone)]
pub struct RemoteDevice {
    pub id: String,
    pub name: String,
    pub model: String,
    pub serial_number: String,
    pub power_kw: Option<f64>,
    pub cloud_status: CloudStatus,
    pub cloud_alarm_count: usize,
    pub discovered_string_count: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct RemoteStringReading {
    pub point: StringPoint,
    pub current: Option<f64>,
    pub voltage: Option<f64>,
}

#[derive(Debug, Clone, Default)]
pub struct RemoteInverterRealtime {
    pub power_kw: Option<f64>,
    pub strings: Vec<RemoteStringReading>,
}

#[derive(Clone)]
pub struct OpenApiClient {
    http: Client,
    config: StoredApiConfiguration,
    secret: String,
}

impl OpenApiClient {
    pub fn from_secure_configuration(config: StoredApiConfiguration) -> AppResult<Self> {
        Ok(Self {
            http: Client::builder().timeout(std::time::Duration::from_secs(30)).build()?,
            secret: SecretStore::api_secret()?,
            config,
        })
    }

    pub async fn exchange_code(&self, code: &str) -> AppResult<OAuthTokens> {
        let response = self
            .http
            .post(format!("{}/openapi/apiManage/token", self.config.region.gateway()))
            .header("x-access-key", &self.secret)
            .header("Content-Type", "application/json")
            .json(&json!({
                "appkey": self.config.app_key,
                "code": code,
                "grant_type": "authorization_code",
                "redirect_uri": self.config.redirect_uri,
            }))
            .send()
            .await?
            .error_for_status()?;
        tokens_from_value(response.json().await?)
    }

    pub async fn ensure_tokens(&self) -> AppResult<OAuthTokens> {
        let tokens = SecretStore::tokens()?.ok_or_else(|| AppError::Configuration("Önce iSolarCloud OAuth yetkilendirmesini tamamlayın".into()))?;
        if tokens.expires_at > Utc::now().timestamp() + 30 {
            return Ok(tokens);
        }
        let response = self
            .http
            .post(format!("{}/openapi/apiManage/refreshToken", self.config.region.gateway()))
            .header("x-access-key", &self.secret)
            .json(&json!({ "appkey": self.config.app_key, "refresh_token": tokens.refresh_token }))
            .send().await?.error_for_status()?;
        let refreshed = tokens_from_value(response.json().await?)?;
        SecretStore::save_tokens(&refreshed)?;
        Ok(refreshed)
    }

    async fn post(&self, path: &str, body: Value) -> AppResult<Value> {
        let tokens = self.ensure_tokens().await?;
        let mut object = body.as_object().cloned().unwrap_or_default();
        object.insert("appkey".into(), Value::String(self.config.app_key.clone()));
        object.insert("lang".into(), Value::String("_en_US".into()));
        let response = self.http
            .post(format!("{}{}", self.config.region.gateway(), path))
            .header("x-access-key", &self.secret)
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .json(&object)
            .send().await?.error_for_status()?;
        let value: Value = response.json().await?;
        if value.get("error").is_some() || matches!(value.get("result_code").and_then(Value::as_str), Some(code) if code != "1") {
            return Err(AppError::Api(redacted_api_error(&value)));
        }
        Ok(value)
    }

    pub async fn plants(&self) -> AppResult<Vec<RemotePlant>> {
        let list = self.post("/openapi/platform/queryPowerStationList", json!({
            "page": 1,
            "size": 100,
            "column_fill_list": ["ps_name", "ps_status", "design_capacity", "current_power", "today_energy"]
        })).await?;
        let rows = list.pointer("/result_data/pageList").and_then(Value::as_array).cloned().unwrap_or_default();
        let mut result = Vec::with_capacity(rows.len());
        for row in rows {
            let id = value_as_string(&row, &["ps_id", "id"]);
            if id.is_empty() { continue; }
            result.push(RemotePlant {
                id,
                name: value_as_string(&row, &["ps_name", "name"]),
                latitude: value_as_f64(&row, &["latitude"]),
                longitude: value_as_f64(&row, &["longitude"]),
                power_kw: measured_kw(&row, &["/dynamic_column/current_power", "/current_power", "/curr_power", "/real_time_power", "/power"]),
                cloud_status: plant_cloud_status(&row),
                cloud_alarm_count: plant_alarm_count(&row),
            });
        }
        Ok(result)
    }

    pub async fn plant_details(&self, plant_ids: &[String]) -> AppResult<Vec<RemotePlant>> {
        if plant_ids.is_empty() { return Ok(Vec::new()); }
        let value = self.post("/openapi/platform/getPowerStationDetail", json!({"ps_ids": plant_ids.join(",")})).await?;
        let rows = value.pointer("/result_data/data_list").and_then(Value::as_array).cloned().unwrap_or_default();
        Ok(rows.into_iter().filter_map(|row| {
            let id = value_as_string(&row, &["ps_id", "id"]);
            (!id.is_empty()).then(|| RemotePlant {
                id,
                name: value_as_string(&row, &["ps_name", "name"]),
                latitude: value_as_f64(&row, &["latitude"]),
                longitude: value_as_f64(&row, &["longitude"]),
                power_kw: measured_kw(&row, &["/dynamic_column/current_power", "/current_power", "/curr_power", "/real_time_power", "/power"]),
                cloud_status: plant_cloud_status(&row),
                cloud_alarm_count: plant_alarm_count(&row),
            })
        }).collect())
    }

    pub async fn devices(&self, plant_id: &str) -> AppResult<Vec<RemoteDevice>> {
        let value = self.post("/openapi/platform/getDeviceListByPsId", json!({
            "ps_id": plant_id, "page": 1, "size": 100, "device_type_list": ["1"]
        })).await?;
        let rows = value.pointer("/result_data/pageList").and_then(Value::as_array).cloned().unwrap_or_default();
        Ok(rows.into_iter().filter_map(|row| {
            let device_type = value_as_i64(&row, &["device_type"]).unwrap_or_default();
            if device_type != 1 { return None; }
            let id = value_as_string(&row, &["ps_key", "uuid", "device_uuid", "id"]);
            (!id.is_empty()).then(|| RemoteDevice {
                id,
                name: value_as_string(&row, &["device_name", "name"]),
                model: value_as_string(&row, &["device_model", "device_model_code", "model"]),
                serial_number: value_as_string(&row, &["sn", "device_sn"]),
                power_kw: measured_kw(&row, &["/dynamic_column/current_power", "/total_active_power", "/active_power", "/device_power", "/power", "/p24"]),
                cloud_status: device_cloud_status(&row),
                cloud_alarm_count: device_alarm_count(&row),
                discovered_string_count: value_as_usize(&row, &["string_count", "string_num", "pv_string_count", "pv_input_count", "input_count", "dc_input_count"]),
            })
        }).collect())
    }

    pub async fn device_realtime_raw(&self, device_keys: &[String], point_ids: &[String]) -> AppResult<Value> {
        let tokens = self.ensure_tokens().await?;
        let response = self.http
            .post(format!("{}/openapi/getDeviceRealTimeData", self.config.region.gateway()))
            .header("x-access-key", &self.secret)
            .header("Authorization", format!("Bearer {}", tokens.access_token))
            .header("sys_code", "901")
            .header("lang", "_en_US")
            .json(&json!({
                "appkey": self.config.app_key,
                "token": tokens.access_token,
                "point_id_list": point_ids,
                "ps_key_list": device_keys,
                "device_type": 1
            }))
            .send().await?.error_for_status()?;
        let value: Value = response.json().await?;
        if value.get("error").is_some() || matches!(value.get("result_code").and_then(Value::as_str), Some(code) if code != "1") {
            return Err(AppError::Api(redacted_api_error(&value)));
        }
        Ok(value)
    }

    pub async fn inverter_realtime(&self, device_keys: &[String]) -> AppResult<BTreeMap<String, RemoteInverterRealtime>> {
        let mut points_by_device = BTreeMap::<String, serde_json::Map<String, Value>>::new();
        for point_ids in inverter_point_batches() {
            for device_batch in device_keys.chunks(20) {
                let raw = self.device_realtime_raw(device_batch, &point_ids).await?;
                for row in raw.pointer("/result_data/device_point_list").and_then(Value::as_array).into_iter().flatten() {
                    let Some(points) = row.get("device_point").unwrap_or(row).as_object() else { continue; };
                    let key = value_as_string(row.get("device_point").unwrap_or(row), &["ps_key"]);
                    if !key.is_empty() {
                        points_by_device.entry(key).or_default().extend(points.clone());
                    }
                }
            }
        }
        Ok(points_by_device.into_iter().map(|(key, points)| {
            let power_kw = point_value(&points, INVERTER_ACTIVE_POWER).map(|value| value / 1000.0);
            let strings = STRING_POINTS.iter().filter_map(|point| {
                let current = point_value(&points, point.current);
                let voltage = point_value(&points, point.voltage);
                if current.is_none() && voltage.is_none() { return None; }
                Some(RemoteStringReading {
                    point: *point,
                    current,
                    voltage,
                })
            }).collect();
            (key, RemoteInverterRealtime { power_kw, strings })
        }).collect())
    }

    pub async fn plant_realtime_power(&self, plant_ids: &[String]) -> AppResult<BTreeMap<String, f64>> {
        let mut result = BTreeMap::new();
        for plant_batch in plant_ids.chunks(20) {
            let keys: Vec<_> = plant_batch.iter().map(|id| format!("{id}_11_0_0")).collect();
            let raw = self.device_realtime_raw(&keys, &[PLANT_ACTIVE_POWER.to_string()]).await?;
            for row in raw.pointer("/result_data/device_point_list").and_then(Value::as_array).into_iter().flatten() {
                let Some(points) = row.get("device_point").unwrap_or(row).as_object() else { continue; };
                let key = points.get("ps_id")
                    .map(|value| value.to_string().trim_matches('"').to_string())
                    .or_else(|| points.get("ps_key").and_then(Value::as_str).and_then(|value| value.split('_').next()).map(str::to_string))
                    .unwrap_or_default();
                if let Some(power_w) = point_value(points, PLANT_ACTIVE_POWER) {
                    result.insert(key, power_w / 1000.0);
                }
            }
        }
        Ok(result)
    }
}

fn point_value(points: &serde_json::Map<String, Value>, point_id: &str) -> Option<f64> {
    points.get(&format!("p{point_id}")).and_then(number_from_value)
}

fn tokens_from_value(value: Value) -> AppResult<OAuthTokens> {
    let access_token = value.get("access_token").and_then(Value::as_str).ok_or_else(|| AppError::Api(redacted_api_error(&value)))?;
    let refresh_token = value.get("refresh_token").and_then(Value::as_str).ok_or_else(|| AppError::Api("OAuth cevabında refresh_token yok".into()))?;
    let expires_in = value.get("expires_in").and_then(Value::as_i64).unwrap_or(3600);
    Ok(OAuthTokens { access_token: access_token.into(), refresh_token: refresh_token.into(), expires_at: Utc::now().timestamp() + expires_in - 20 })
}

fn redacted_api_error(value: &Value) -> String {
    value.get("error_description").or_else(|| value.get("result_msg")).or_else(|| value.get("error"))
        .and_then(Value::as_str).unwrap_or("Bilinmeyen API cevabı").to_string()
}

fn value_as_string(value: &Value, keys: &[&str]) -> String {
    keys.iter().find_map(|key| value.get(key)).map(|item| item.as_str().map(str::to_string).unwrap_or_else(|| item.to_string().trim_matches('"').to_string())).unwrap_or_default()
}
fn value_as_f64(value: &Value, keys: &[&str]) -> Option<f64> {
    keys.iter().find_map(|key| value.get(key)).and_then(|item| item.as_f64().or_else(|| item.as_str()?.parse().ok()))
}
fn value_as_i64(value: &Value, keys: &[&str]) -> Option<i64> {
    keys.iter().find_map(|key| value.get(key)).and_then(|item| item.as_i64().or_else(|| item.as_str()?.parse().ok()))
}
fn value_as_usize(value: &Value, keys: &[&str]) -> Option<usize> {
    value_as_i64(value, keys).and_then(|item| usize::try_from(item).ok()).filter(|item| *item > 0)
}

fn measured_kw(value: &Value, pointers: &[&str]) -> Option<f64> {
    pointers.iter().find_map(|pointer| value.pointer(pointer).and_then(measurement_to_kw))
}

fn measurement_to_kw(value: &Value) -> Option<f64> {
    let (number, unit) = if let Some(object) = value.as_object() {
        let number = ["value", "val", "data"].iter().find_map(|key| object.get(*key)).and_then(number_from_value)?;
        let unit = ["unit", "unit_name"].iter().find_map(|key| object.get(*key)).and_then(Value::as_str).unwrap_or("");
        (number, unit)
    } else if let Some(raw) = value.as_str() {
        let number = raw.split_whitespace().next()?.replace(',', ".").parse().ok()?;
        let unit = raw.split_whitespace().nth(1).unwrap_or("");
        (number, unit)
    } else {
        (value.as_f64()?, "")
    };
    let unit = unit.to_ascii_lowercase();
    Some(if unit == "mw" { number * 1000.0 } else if unit == "w" { number / 1000.0 } else if unit.is_empty() && number.abs() > 10_000.0 { number / 1000.0 } else { number })
}

fn number_from_value(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.replace(',', ".").parse().ok())
}

fn plant_cloud_status(value: &Value) -> CloudStatus {
    match value_as_i64(value, &["ps_fault_status"]) {
        Some(1) => return CloudStatus::Fault,
        Some(2) => return CloudStatus::Alarm,
        Some(3) => return CloudStatus::Normal,
        _ => {}
    }
    match value_as_i64(value, &["online_status", "ps_status", "status"]) {
        Some(0) => CloudStatus::Offline,
        Some(1) => CloudStatus::Normal,
        Some(3) => CloudStatus::Commissioning,
        Some(4) => CloudStatus::Fault,
        Some(5) => CloudStatus::Alarm,
        _ => CloudStatus::Unknown,
    }
}

fn plant_alarm_count(value: &Value) -> usize {
    value_as_usize(value, &["alarm_count", "fault_count", "warning_count"])
        .or_else(|| value.pointer("/fault_map/more_count").and_then(number_from_value).map(|count| count.max(0.0) as usize + 1))
        .unwrap_or_else(|| usize::from(matches!(plant_cloud_status(value), CloudStatus::Alarm | CloudStatus::Fault)))
}

fn device_cloud_status(value: &Value) -> CloudStatus {
    if value_as_i64(value, &["dev_status", "device_status"]) == Some(0) {
        return CloudStatus::Offline;
    }
    match value_as_i64(value, &["dev_fault_status", "fault_status", "device_state", "status"]) {
        Some(1) => CloudStatus::Fault,
        Some(2) => CloudStatus::Alarm,
        Some(4) => CloudStatus::Normal,
        Some(6) => CloudStatus::Commissioning,
        _ => CloudStatus::Unknown,
    }
}

fn device_alarm_count(value: &Value) -> usize {
    value_as_usize(value, &["alarm_count", "fault_count", "warning_count"])
        .unwrap_or_else(|| usize::from(matches!(device_cloud_status(value), CloudStatus::Alarm | CloudStatus::Fault)))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn parses_nested_plant_power_and_alarm_from_official_shape() {
        let row = json!({
            "ps_status": 5,
            "fault_map": {"more_count": 0},
            "dynamic_column": {"current_power": {"value": "1.37", "unit": "MW"}}
        });
        assert_eq!(measured_kw(&row, &["/dynamic_column/current_power"]), Some(1370.0));
        assert_eq!(plant_cloud_status(&row), CloudStatus::Alarm);
        assert_eq!(plant_alarm_count(&row), 1);
    }

    #[test]
    fn parses_device_status_power_and_discovered_strings() {
        let row = json!({"device_status": 1, "fault_status": 2, "total_active_power": "197.90 kW", "string_num": "24"});
        assert_eq!(measured_kw(&row, &["/total_active_power"]), Some(197.9));
        assert_eq!(device_cloud_status(&row), CloudStatus::Alarm);
        assert_eq!(device_alarm_count(&row), 1);
        assert_eq!(value_as_usize(&row, &["string_num"]), Some(24));
    }
}
