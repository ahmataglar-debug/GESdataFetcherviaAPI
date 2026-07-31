use chrono::Utc;
use reqwest::Client;
use serde_json::{json, Value};

use crate::{
    domain::StoredApiConfiguration,
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
}

#[derive(Debug, Clone)]
pub struct RemoteDevice {
    pub id: String,
    pub name: String,
    pub model: String,
    pub serial_number: String,
    pub power_kw: Option<f64>,
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
        let list = self.post("/openapi/platform/queryPowerStationList", json!({"page": 1, "size": 100})).await?;
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
                power_kw: value_as_f64(&row, &["curr_power", "power"]).map(normalize_kw),
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
                power_kw: value_as_f64(&row, &["curr_power", "power"]).map(normalize_kw),
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
                power_kw: value_as_f64(&row, &["active_power", "power"]).map(normalize_kw),
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
fn normalize_kw(value: f64) -> f64 { if value.abs() > 10_000.0 { value / 1000.0 } else { value } }
