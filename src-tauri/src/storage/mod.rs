use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    domain::{
        ConnectionState, DashboardSnapshot, InverterSummary, PlantSchemaUpdate, PlantSummary,
        Severity, StoredApiConfiguration, StringReading, TimezoneSource,
    },
    error::AppResult,
};

#[derive(Clone)]
pub struct Repository {
    path: PathBuf,
}

impl Repository {
    pub fn new(path: impl AsRef<Path>) -> AppResult<Self> {
        let repository = Self {
            path: path.as_ref().to_path_buf(),
        };
        repository.migrate()?;
        Ok(repository)
    }

    fn connect(&self) -> AppResult<Connection> {
        let connection = Connection::open(&self.path)?;
        connection.pragma_update(None, "foreign_keys", "ON")?;
        connection.pragma_update(None, "journal_mode", "WAL")?;
        Ok(connection)
    }

    fn migrate(&self) -> AppResult<()> {
        let connection = self.connect()?;
        connection.execute_batch(include_str!("schema.sql"))?;
        Ok(())
    }

    pub fn save_api_configuration(&self, config: &StoredApiConfiguration) -> AppResult<()> {
        let value = serde_json::to_string(config).expect("stored API configuration is serializable");
        self.connect()?.execute(
            "INSERT INTO settings(key, value) VALUES('api_configuration', ?1)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            [value],
        )?;
        Ok(())
    }

    pub fn api_configuration(&self) -> AppResult<Option<StoredApiConfiguration>> {
        let value: Option<String> = self
            .connect()?
            .query_row(
                "SELECT value FROM settings WHERE key = 'api_configuration'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        Ok(value.and_then(|raw| serde_json::from_str(&raw).ok()))
    }

    pub fn mark_sync(&self, completed_at: DateTime<Utc>, next_sync_at: DateTime<Utc>) -> AppResult<()> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO scheduler_state(id, last_attempt_at, last_success_at, next_sync_at)
             VALUES(1, ?1, ?1, ?2)
             ON CONFLICT(id) DO UPDATE SET
               last_attempt_at = excluded.last_attempt_at,
               last_success_at = excluded.last_success_at,
               next_sync_at = excluded.next_sync_at",
            params![completed_at.to_rfc3339(), next_sync_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn upsert_plant(
        &self,
        id: &str,
        name: &str,
        latitude: Option<f64>,
        longitude: Option<f64>,
        timezone: &str,
        timezone_source: &str,
        power_kw: Option<f64>,
    ) -> AppResult<()> {
        self.connect()?.execute(
            "INSERT INTO plants(id, name, latitude, longitude, timezone, timezone_source, power_kw, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, latitude=excluded.latitude,
               longitude=excluded.longitude, timezone=excluded.timezone,
               timezone_source=excluded.timezone_source, power_kw=excluded.power_kw,
               updated_at=excluded.updated_at",
            params![id, name, latitude, longitude, timezone, timezone_source, power_kw, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn upsert_inverter(
        &self,
        id: &str,
        plant_id: &str,
        name: &str,
        model: &str,
        serial_number: &str,
        power_kw: Option<f64>,
    ) -> AppResult<()> {
        self.connect()?.execute(
            "INSERT INTO inverters(id, plant_id, name, model, serial_number, power_kw, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7)
             ON CONFLICT(id) DO UPDATE SET plant_id=excluded.plant_id, name=excluded.name,
               model=excluded.model, serial_number=excluded.serial_number,
               power_kw=excluded.power_kw, updated_at=excluded.updated_at",
            params![id, plant_id, name, model, serial_number, power_kw, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn save_plant_schema(&self, schema: &PlantSchemaUpdate) -> AppResult<()> {
        let mut connection = self.connect()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "INSERT INTO plant_schemas(plant_id, current_zero_threshold, voltage_zero_threshold, updated_at)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(plant_id) DO UPDATE SET current_zero_threshold=excluded.current_zero_threshold,
               voltage_zero_threshold=excluded.voltage_zero_threshold, updated_at=excluded.updated_at",
            params![schema.plant_id, schema.current_zero_threshold, schema.voltage_zero_threshold, Utc::now().to_rfc3339()],
        )?;
        for item in &schema.strings {
            transaction.execute(
                "UPDATE strings SET connected = ?2 WHERE id = ?1",
                params![item.id, item.connected as i32],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn dashboard_snapshot(&self, configured: bool, connected: bool) -> AppResult<DashboardSnapshot> {
        let connection = self.connect()?;
        let scheduler = connection
            .query_row(
                "SELECT last_success_at, next_sync_at FROM scheduler_state WHERE id = 1",
                [],
                |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()?
            .unwrap_or((None, None));

        let mut plant_statement = connection.prepare(
            "SELECT p.id, p.name, p.latitude, p.longitude, p.timezone, p.timezone_source,
                    p.power_kw, EXISTS(SELECT 1 FROM plant_schemas s WHERE s.plant_id = p.id)
             FROM plants p ORDER BY p.name",
        )?;
        let plant_rows = plant_statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<f64>>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                row.get::<_, Option<f64>>(6)?, row.get::<_, bool>(7)?,
            ))
        })?;

        let mut plants = Vec::new();
        for row in plant_rows {
            let (id, name, latitude, longitude, timezone, source, power_kw, schema_configured) = row?;
            let inverters = Self::inverters_for(&connection, &id)?;
            let string_count = inverters.iter().map(|item| item.strings.len()).sum();
            let alert_count = inverters.iter().flat_map(|item| &item.strings).filter(|item| matches!(item.severity, Severity::Suspicious | Severity::Critical)).count();
            let status = aggregate_status(inverters.iter().map(|item| &item.status));
            plants.push(PlantSummary {
                id,
                name,
                status,
                power_kw,
                inverter_count: inverters.len(),
                string_count,
                alert_count,
                schema_configured,
                timezone,
                timezone_source: parse_timezone_source(&source),
                latitude,
                longitude,
                inverters,
            });
        }

        Ok(DashboardSnapshot {
            connection_state: if !configured { ConnectionState::NotConfigured } else if connected { ConnectionState::Connected } else { ConnectionState::Authorizing },
            last_sync_at: scheduler.0.and_then(|value| DateTime::parse_from_rfc3339(&value).ok()).map(|value| value.with_timezone(&Utc)),
            next_sync_at: scheduler.1.and_then(|value| DateTime::parse_from_rfc3339(&value).ok()).map(|value| value.with_timezone(&Utc)),
            plants,
            warning: None,
        })
    }

    fn inverters_for(connection: &Connection, plant_id: &str) -> AppResult<Vec<InverterSummary>> {
        let mut statement = connection.prepare(
            "SELECT id, name, model, serial_number, power_kw FROM inverters WHERE plant_id = ?1 ORDER BY name",
        )?;
        let rows = statement.query_map([plant_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, Option<f64>>(4)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (id, name, model, serial_number, power_kw) = row?;
            let strings = Self::strings_for(connection, &id)?;
            let status = aggregate_status(strings.iter().map(|item| &item.severity));
            result.push(InverterSummary { id, name, model, serial_number, status, power_kw, strings });
        }
        Ok(result)
    }

    fn strings_for(connection: &Connection, inverter_id: &str) -> AppResult<Vec<StringReading>> {
        let mut statement = connection.prepare(
            "SELECT s.id, s.label, s.connected, m.current, m.voltage, m.sampled_at,
                    COALESCE(a.severity, CASE WHEN EXISTS(SELECT 1 FROM plant_schemas ps
                      JOIN inverters i ON i.plant_id = ps.plant_id WHERE i.id = s.inverter_id)
                      THEN 'learning' ELSE 'unconfigured' END),
                    COALESCE(a.reason, '30 geçerli gündüz örneği bekleniyor'),
                    (SELECT COUNT(DISTINCT date(sampled_at)) FROM measurements h
                      WHERE h.string_id = s.id AND h.is_valid_daylight = 1)
             FROM strings s
             LEFT JOIN measurements m ON m.id = (SELECT id FROM measurements lm WHERE lm.string_id=s.id ORDER BY sampled_at DESC LIMIT 1)
             LEFT JOIN analysis_results a ON a.id = (SELECT id FROM analysis_results la WHERE la.string_id=s.id ORDER BY analyzed_at DESC LIMIT 1)
             WHERE s.inverter_id = ?1 ORDER BY s.position",
        )?;
        let rows = statement.query_map([inverter_id], |row| {
            let severity: String = row.get(6)?;
            Ok(StringReading {
                id: row.get(0)?, label: row.get(1)?, connected: row.get(2)?, current: row.get(3)?, voltage: row.get(4)?,
                sampled_at: row.get::<_, Option<String>>(5)?.and_then(|value| DateTime::parse_from_rfc3339(&value).ok()).map(|value| value.with_timezone(&Utc)),
                severity: parse_severity(&severity), reason: row.get(7)?, learned_days: row.get::<_, i64>(8)? as usize,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

fn parse_severity(value: &str) -> Severity {
    match value { "normal" => Severity::Normal, "suspicious" => Severity::Suspicious, "critical" => Severity::Critical, "night" => Severity::Night, "no_data" => Severity::NoData, "unconfigured" => Severity::Unconfigured, _ => Severity::Learning }
}

fn parse_timezone_source(value: &str) -> TimezoneSource {
    match value { "coordinates" => TimezoneSource::Coordinates, "online_lookup" => TimezoneSource::OnlineLookup, "turkey_default" => TimezoneSource::TurkeyDefault, _ => TimezoneSource::Unknown }
}

fn aggregate_status<'a>(statuses: impl Iterator<Item = &'a Severity>) -> Severity {
    let mut best = Severity::Normal;
    for status in statuses {
        best = match (&best, status) {
            (_, Severity::Critical) => Severity::Critical,
            (Severity::Critical, _) => Severity::Critical,
            (_, Severity::Suspicious) => Severity::Suspicious,
            (Severity::Suspicious, _) => Severity::Suspicious,
            (_, Severity::NoData) => Severity::NoData,
            (Severity::NoData, _) => Severity::NoData,
            (_, Severity::Unconfigured) => Severity::Unconfigured,
            (Severity::Unconfigured, _) => Severity::Unconfigured,
            (_, Severity::Learning) => Severity::Learning,
            _ => best,
        };
    }
    best
}

