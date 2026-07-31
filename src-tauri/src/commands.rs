use tauri::State;

use crate::{
    api::OAuthService,
    domain::{ApiConfigurationInput, DashboardSnapshot, PlantSchemaUpdate, StoredApiConfiguration},
    error::{AppError, AppResult},
    secrets::SecretStore,
    sync_service::{connection_is_ready, synchronize},
    AppState,
};

#[tauri::command]
pub fn get_dashboard_snapshot(state: State<'_, AppState>) -> AppResult<DashboardSnapshot> {
    let configured = state.repository.api_configuration()?.is_some();
    state.repository.dashboard_snapshot(configured, connection_is_ready(&state.repository))
}

#[tauri::command]
pub fn save_api_configuration(state: State<'_, AppState>, config: ApiConfigurationInput) -> AppResult<()> {
    if config.app_key.trim().is_empty() || config.application_id.trim().is_empty() || config.secret_key.trim().is_empty() {
        return Err(AppError::Configuration("AppKey, Secret key ve Application ID zorunludur".into()));
    }
    let redirect = url::Url::parse(&config.redirect_uri)?;
    if redirect.host_str() != Some("127.0.0.1") && redirect.host_str() != Some("localhost") {
        return Err(AppError::Configuration("Redirect URI yalnızca yerel callback olmalıdır".into()));
    }
    SecretStore::save_api_secret(config.secret_key.trim())?;
    state.repository.save_api_configuration(&StoredApiConfiguration {
        app_key: config.app_key.trim().into(),
        application_id: config.application_id.trim().into(),
        region: config.region,
        redirect_uri: config.redirect_uri,
    })
}

#[tauri::command]
pub async fn begin_oauth(state: State<'_, AppState>) -> AppResult<()> {
    let config = state.repository.api_configuration()?.ok_or_else(|| AppError::Configuration("Önce OpenAPI ayarlarını kaydedin".into()))?;
    OAuthService::authorize(config).await
}

#[tauri::command]
pub async fn sync_now(state: State<'_, AppState>) -> AppResult<DashboardSnapshot> {
    synchronize(&state.repository, false).await
}

#[tauri::command]
pub fn save_plant_schema(state: State<'_, AppState>, schema: PlantSchemaUpdate) -> AppResult<()> {
    if schema.current_zero_threshold < 0.0 || schema.voltage_zero_threshold < 0.0 {
        return Err(AppError::Configuration("Sıfır eşikleri negatif olamaz".into()));
    }
    if schema.strings.iter().any(|item| {
        (!item.current_point_id.is_empty() && !item.current_point_id.chars().all(|value| value.is_ascii_digit()))
            || (!item.voltage_point_id.is_empty() && !item.voltage_point_id.chars().all(|value| value.is_ascii_digit()))
    }) {
        return Err(AppError::Configuration("Akım ve gerilim point ID alanları yalnızca rakam içerebilir".into()));
    }
    state.repository.save_plant_schema(&schema)
}
