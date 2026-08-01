use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub const REQUIRED_BASELINE_DAYS: usize = 30;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CloudStatus {
    Normal,
    Alarm,
    Fault,
    Offline,
    Commissioning,
    Unknown,
}

impl CloudStatus {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Alarm => "alarm",
            Self::Fault => "fault",
            Self::Offline => "offline",
            Self::Commissioning => "commissioning",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    Normal,
    Suspicious,
    Critical,
    Learning,
    Night,
    NoData,
    Disconnected,
    Unconfigured,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Suspicious => "suspicious",
            Self::Critical => "critical",
            Self::Learning => "learning",
            Self::Night => "night",
            Self::NoData => "no_data",
            Self::Disconnected => "disconnected",
            Self::Unconfigured => "unconfigured",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StringReading {
    pub id: String,
    pub label: String,
    pub current: Option<f64>,
    pub voltage: Option<f64>,
    pub connected: bool,
    pub current_point_id: String,
    pub voltage_point_id: String,
    pub severity: Severity,
    pub reason: String,
    pub learned_days: usize,
    pub sampled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InverterSummary {
    pub id: String,
    pub name: String,
    pub model: String,
    pub serial_number: String,
    pub status: Severity,
    pub cloud_status: CloudStatus,
    pub cloud_alarm_count: usize,
    pub power_kw: Option<f64>,
    pub discovered_string_count: Option<usize>,
    pub learned_days: usize,
    pub days_until_analysis: usize,
    pub strings: Vec<StringReading>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TimezoneSource {
    Coordinates,
    OnlineLookup,
    TurkeyDefault,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantSummary {
    pub id: String,
    pub name: String,
    pub status: Severity,
    pub cloud_status: CloudStatus,
    pub cloud_alarm_count: usize,
    pub power_kw: Option<f64>,
    pub inverter_count: usize,
    pub string_count: Option<usize>,
    pub alert_count: usize,
    pub learned_days: usize,
    pub days_until_analysis: usize,
    pub schema_configured: bool,
    pub timezone: String,
    pub timezone_source: TimezoneSource,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub inverters: Vec<InverterSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionState {
    NotConfigured,
    Authorizing,
    Connected,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DashboardSnapshot {
    pub connection_state: ConnectionState,
    pub last_sync_at: Option<DateTime<Utc>>,
    pub next_sync_at: Option<DateTime<Utc>>,
    pub plants: Vec<PlantSummary>,
    pub warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApiRegion {
    Europe,
    International,
    China,
    Australia,
}

impl ApiRegion {
    pub fn gateway(&self) -> &'static str {
        match self {
            Self::Europe => "https://gateway.isolarcloud.eu",
            Self::International => "https://gateway.isolarcloud.com.hk",
            Self::China => "https://gateway.isolarcloud.com",
            Self::Australia => "https://augateway.isolarcloud.com",
        }
    }

    pub fn authorization_site(&self) -> (&'static str, u8) {
        match self {
            Self::Europe => ("https://web3.isolarcloud.eu", 3),
            Self::International => ("https://web3.isolarcloud.com.hk", 2),
            Self::China => ("https://web3.isolarcloud.com", 1),
            Self::Australia => ("https://auweb3.isolarcloud.com", 7),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiConfigurationInput {
    pub app_key: String,
    pub secret_key: String,
    pub application_id: String,
    pub region: ApiRegion,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredApiConfiguration {
    pub app_key: String,
    pub application_id: String,
    pub region: ApiRegion,
    pub redirect_uri: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlantSchemaUpdate {
    pub plant_id: String,
    pub current_zero_threshold: f64,
    pub voltage_zero_threshold: f64,
    pub strings: Vec<StringConnectionUpdate>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StringConnectionUpdate {
    pub id: String,
    pub inverter_id: String,
    pub position: i64,
    pub label: String,
    pub current_point_id: String,
    pub voltage_point_id: String,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct MappedString {
    pub id: String,
    pub inverter_id: String,
    pub current_point_id: String,
    pub voltage_point_id: String,
    pub connected: bool,
    pub timezone: String,
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub current_zero_threshold: f64,
    pub voltage_zero_threshold: f64,
    pub schema_configured: bool,
}

#[derive(Debug, Clone)]
pub struct Measurement {
    pub string_id: String,
    pub current: Option<f64>,
    pub voltage: Option<f64>,
    pub sampled_at: DateTime<Utc>,
    pub is_valid_daylight: bool,
}
