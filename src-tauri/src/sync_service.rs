use chrono::{Duration, Utc};

use crate::{
    api::OpenApiClient,
    domain::DashboardSnapshot,
    error::{AppError, AppResult},
    secrets::SecretStore,
    solar_time::inferred_timezone,
    storage::Repository,
};

pub async fn synchronize(repository: &Repository) -> AppResult<DashboardSnapshot> {
    let config = repository.api_configuration()?.ok_or_else(|| AppError::Configuration("OpenAPI yapılandırması henüz kaydedilmedi".into()))?;
    let client = OpenApiClient::from_secure_configuration(config)?;
    let plants = client.plants().await?;
    let plant_ids: Vec<_> = plants.iter().map(|plant| plant.id.clone()).collect();
    let details = client.plant_details(&plant_ids).await.unwrap_or_default();

    for listed in plants {
        let detail = details.iter().find(|item| item.id == listed.id);
        let latitude = detail.and_then(|item| item.latitude).or(listed.latitude);
        let longitude = detail.and_then(|item| item.longitude).or(listed.longitude);
        let (timezone, timezone_source) = inferred_timezone(latitude, longitude);
        let name = detail.filter(|item| !item.name.is_empty()).map(|item| item.name.as_str()).unwrap_or(&listed.name);
        let power_kw = detail.and_then(|item| item.power_kw).or(listed.power_kw);
        repository.upsert_plant(&listed.id, name, latitude, longitude, timezone, timezone_source, power_kw)?;
        for device in client.devices(&listed.id).await? {
            repository.upsert_inverter(&device.id, &listed.id, &device.name, &device.model, &device.serial_number, device.power_kw)?;
        }
    }

    let now = Utc::now();
    repository.mark_sync(now, now + Duration::hours(24))?;
    repository.dashboard_snapshot(true, true)
}

pub fn connection_is_ready(repository: &Repository) -> bool {
    repository.api_configuration().ok().flatten().is_some() && SecretStore::tokens().ok().flatten().is_some()
}

