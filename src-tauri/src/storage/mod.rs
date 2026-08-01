use std::path::{Path, PathBuf};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};

use crate::{
    domain::{
        ConnectionState, DashboardSnapshot, InverterSummary, MappedString, Measurement,
        CloudStatus, PlantSchemaUpdate, PlantSummary, Severity, StoredApiConfiguration, StringReading,
        TimezoneSource,
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
        ensure_column(&connection, "plants", "cloud_status", "TEXT NOT NULL DEFAULT 'unknown'")?;
        ensure_column(&connection, "plants", "cloud_alarm_count", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&connection, "inverters", "cloud_status", "TEXT NOT NULL DEFAULT 'unknown'")?;
        ensure_column(&connection, "inverters", "cloud_alarm_count", "INTEGER NOT NULL DEFAULT 0")?;
        ensure_column(&connection, "inverters", "discovered_string_count", "INTEGER")?;
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

    pub fn mark_sync(&self, completed_at: DateTime<Utc>, next_sync_at: DateTime<Utc>, was_night: bool) -> AppResult<()> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO scheduler_state(id, last_attempt_at, last_success_at, next_sync_at, last_attempt_was_night)
             VALUES(1, ?1, ?1, ?2, ?3)
             ON CONFLICT(id) DO UPDATE SET
               last_attempt_at = excluded.last_attempt_at,
               last_success_at = excluded.last_success_at,
               next_sync_at = excluded.next_sync_at,
               last_attempt_was_night = excluded.last_attempt_was_night",
            params![completed_at.to_rfc3339(), next_sync_at.to_rfc3339(), was_night as i32],
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
        cloud_status: &CloudStatus,
        cloud_alarm_count: usize,
    ) -> AppResult<()> {
        self.connect()?.execute(
            "INSERT INTO plants(id, name, latitude, longitude, timezone, timezone_source, power_kw, cloud_status, cloud_alarm_count, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET name=excluded.name, latitude=excluded.latitude,
               longitude=excluded.longitude, timezone=excluded.timezone,
               timezone_source=excluded.timezone_source, power_kw=excluded.power_kw,
               cloud_status=excluded.cloud_status, cloud_alarm_count=excluded.cloud_alarm_count,
               updated_at=excluded.updated_at",
            params![id, name, latitude, longitude, timezone, timezone_source, power_kw, cloud_status.as_str(), cloud_alarm_count as i64, Utc::now().to_rfc3339()],
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
        cloud_status: &CloudStatus,
        cloud_alarm_count: usize,
        discovered_string_count: Option<usize>,
    ) -> AppResult<()> {
        self.connect()?.execute(
            "INSERT INTO inverters(id, plant_id, name, model, serial_number, power_kw, cloud_status, cloud_alarm_count, discovered_string_count, updated_at)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
             ON CONFLICT(id) DO UPDATE SET plant_id=excluded.plant_id, name=excluded.name,
               model=excluded.model, serial_number=excluded.serial_number,
               power_kw=excluded.power_kw, cloud_status=excluded.cloud_status,
               cloud_alarm_count=excluded.cloud_alarm_count,
               discovered_string_count=COALESCE(excluded.discovered_string_count, inverters.discovered_string_count),
               updated_at=excluded.updated_at",
            params![id, plant_id, name, model, serial_number, power_kw, cloud_status.as_str(), cloud_alarm_count as i64, discovered_string_count.map(|value| value as i64), Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn update_plant_power(&self, plant_id: &str, power_kw: f64) -> AppResult<()> {
        self.connect()?.execute(
            "UPDATE plants SET power_kw=?2, updated_at=?3 WHERE id=?1",
            params![plant_id, power_kw, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn update_inverter_realtime(&self, inverter_id: &str, power_kw: Option<f64>, string_count: usize) -> AppResult<()> {
        self.connect()?.execute(
            "UPDATE inverters SET power_kw=COALESCE(?2, power_kw), discovered_string_count=?3, updated_at=?4 WHERE id=?1",
            params![inverter_id, power_kw, string_count as i64, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn upsert_discovered_string(
        &self,
        inverter_id: &str,
        position: usize,
        current_point_id: &str,
        voltage_point_id: &str,
    ) -> AppResult<String> {
        let id = format!("{inverter_id}:string:{position}");
        self.connect()?.execute(
            "INSERT INTO strings(id, inverter_id, position, label, current_point_id, voltage_point_id, connected)
             VALUES(?1, ?2, ?3, ?4, ?5, ?6, 1)
             ON CONFLICT(inverter_id, position) DO UPDATE SET
               current_point_id=COALESCE(strings.current_point_id, excluded.current_point_id),
               voltage_point_id=COALESCE(strings.voltage_point_id, excluded.voltage_point_id)",
            params![id, inverter_id, position as i64, format!("String {position}"), current_point_id, voltage_point_id],
        )?;
        Ok(id)
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
                "INSERT INTO strings(id, inverter_id, position, label, current_point_id, voltage_point_id, connected)
                 VALUES(?1, ?2, ?3, ?4, NULLIF(?5, ''), NULLIF(?6, ''), ?7)
                 ON CONFLICT(id) DO UPDATE SET inverter_id=excluded.inverter_id,
                   position=excluded.position, label=excluded.label,
                   current_point_id=excluded.current_point_id,
                   voltage_point_id=excluded.voltage_point_id, connected=excluded.connected",
                params![item.id, item.inverter_id, item.position, item.label, item.current_point_id, item.voltage_point_id, item.connected as i32],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn mapped_strings(&self) -> AppResult<Vec<MappedString>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT s.id, s.inverter_id, s.current_point_id,
                    s.voltage_point_id, s.connected, p.timezone, p.latitude, p.longitude,
                    COALESCE(ps.current_zero_threshold, 0.15),
                    COALESCE(ps.voltage_zero_threshold, 10.0), ps.plant_id IS NOT NULL
             FROM strings s
             JOIN inverters i ON i.id = s.inverter_id
             JOIN plants p ON p.id = i.plant_id
             LEFT JOIN plant_schemas ps ON ps.plant_id = p.id
             WHERE s.current_point_id IS NOT NULL AND s.current_point_id != ''
               AND s.voltage_point_id IS NOT NULL AND s.voltage_point_id != ''
             ORDER BY s.inverter_id, s.position",
        )?;
        let rows = statement.query_map([], |row| Ok(MappedString {
            id: row.get(0)?, inverter_id: row.get(1)?, current_point_id: row.get(2)?, voltage_point_id: row.get(3)?, connected: row.get(4)?,
            timezone: row.get(5)?, latitude: row.get(6)?, longitude: row.get(7)?,
            current_zero_threshold: row.get(8)?, voltage_zero_threshold: row.get(9)?, schema_configured: row.get(10)?,
        }))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn save_measurement(&self, measurement: &Measurement) -> AppResult<i64> {
        let connection = self.connect()?;
        connection.execute(
            "INSERT INTO measurements(string_id, current, voltage, sampled_at, is_valid_daylight)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![measurement.string_id, measurement.current, measurement.voltage, measurement.sampled_at.to_rfc3339(), measurement.is_valid_daylight as i32],
        )?;
        Ok(connection.last_insert_rowid())
    }

    pub fn save_live_reading(&self, measurement: &Measurement) -> AppResult<()> {
        self.connect()?.execute(
            "INSERT INTO latest_readings(string_id, current, voltage, sampled_at)
             VALUES(?1, ?2, ?3, ?4)
             ON CONFLICT(string_id) DO UPDATE SET current=excluded.current,
               voltage=excluded.voltage, sampled_at=excluded.sampled_at",
            params![measurement.string_id, measurement.current, measurement.voltage, measurement.sampled_at.to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn measurement_history(&self, string_id: &str, before: DateTime<Utc>, limit: usize) -> AppResult<Vec<Measurement>> {
        let connection = self.connect()?;
        let mut statement = connection.prepare(
            "SELECT current, voltage, sampled_at, is_valid_daylight
             FROM measurements WHERE string_id=?1 AND sampled_at < ?2 AND is_valid_daylight=1
             ORDER BY sampled_at DESC LIMIT ?3",
        )?;
        let rows = statement.query_map(params![string_id, before.to_rfc3339(), limit as i64], |row| {
            let raw: String = row.get(2)?;
            Ok(Measurement {
                string_id: string_id.to_string(), current: row.get(0)?, voltage: row.get(1)?,
                sampled_at: DateTime::parse_from_rfc3339(&raw).map(|value| value.with_timezone(&Utc)).unwrap_or(before),
                is_valid_daylight: row.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn save_analysis(&self, string_id: &str, measurement_id: i64, severity: &Severity, reason: &str) -> AppResult<()> {
        self.connect()?.execute(
            "INSERT INTO analysis_results(string_id, measurement_id, severity, reason, analyzed_at)
             VALUES(?1, ?2, ?3, ?4, ?5)",
            params![string_id, measurement_id, severity.as_str(), reason, Utc::now().to_rfc3339()],
        )?;
        Ok(())
    }

    pub fn scheduler_state(&self) -> AppResult<(Option<DateTime<Utc>>, bool)> {
        let row = self.connect()?.query_row(
            "SELECT last_attempt_at, last_attempt_was_night FROM scheduler_state WHERE id=1",
            [],
            |row| Ok((row.get::<_, Option<String>>(0)?, row.get::<_, bool>(1)?)),
        ).optional()?.unwrap_or((None, false));
        Ok((row.0.and_then(|value| DateTime::parse_from_rfc3339(&value).ok()).map(|value| value.with_timezone(&Utc)), row.1))
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
                    p.power_kw, EXISTS(SELECT 1 FROM plant_schemas s WHERE s.plant_id = p.id),
                    p.cloud_status, p.cloud_alarm_count
             FROM plants p ORDER BY p.name",
        )?;
        let plant_rows = plant_statement.query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, Option<f64>>(2)?,
                row.get::<_, Option<f64>>(3)?, row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                row.get::<_, Option<f64>>(6)?, row.get::<_, bool>(7)?,
                row.get::<_, String>(8)?, row.get::<_, i64>(9)?,
            ))
        })?;

        let mut plants = Vec::new();
        for row in plant_rows {
            let (id, name, latitude, longitude, timezone, source, power_kw, schema_configured, cloud_status, cloud_alarm_count) = row?;
            let inverters = Self::inverters_for(&connection, &id)?;
            let configured_string_count: usize = inverters.iter().map(|item| item.strings.len()).sum();
            let discovered_string_count: usize = inverters.iter().filter_map(|item| item.discovered_string_count).sum();
            let string_count = if configured_string_count > 0 { Some(configured_string_count) } else if discovered_string_count > 0 { Some(discovered_string_count) } else { None };
            let alert_count = inverters.iter().flat_map(|item| &item.strings).filter(|item| matches!(item.severity, Severity::Suspicious | Severity::Critical)).count();
            let status = if !schema_configured { Severity::Unconfigured } else { aggregate_status(inverters.iter().map(|item| &item.status)) };
            let learned_days = inverters.iter().filter(|item| !item.strings.is_empty()).map(|item| item.learned_days).min().unwrap_or(0);
            plants.push(PlantSummary {
                id,
                name,
                status,
                cloud_status: parse_cloud_status(&cloud_status),
                cloud_alarm_count: cloud_alarm_count.max(0) as usize,
                power_kw,
                inverter_count: inverters.len(),
                string_count,
                alert_count,
                learned_days,
                days_until_analysis: crate::domain::REQUIRED_BASELINE_DAYS.saturating_sub(learned_days),
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
            "SELECT id, name, model, serial_number, power_kw, cloud_status, cloud_alarm_count, discovered_string_count
             FROM inverters WHERE plant_id = ?1 ORDER BY name",
        )?;
        let rows = statement.query_map([plant_id], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, Option<f64>>(4)?, row.get::<_, String>(5)?, row.get::<_, i64>(6)?, row.get::<_, Option<i64>>(7)?))
        })?;
        let mut result = Vec::new();
        for row in rows {
            let (id, name, model, serial_number, power_kw, cloud_status, cloud_alarm_count, discovered_string_count) = row?;
            let strings = Self::strings_for(connection, &id)?;
            let status = if strings.is_empty() { Severity::Unconfigured } else { aggregate_status(strings.iter().map(|item| &item.severity)) };
            let learned_days = strings.iter().filter(|item| item.connected).map(|item| item.learned_days).min().unwrap_or(0);
            result.push(InverterSummary {
                id, name, model, serial_number, status,
                cloud_status: parse_cloud_status(&cloud_status),
                cloud_alarm_count: cloud_alarm_count.max(0) as usize,
                power_kw,
                discovered_string_count: discovered_string_count.and_then(|value| usize::try_from(value).ok()).filter(|value| *value > 0),
                learned_days,
                days_until_analysis: crate::domain::REQUIRED_BASELINE_DAYS.saturating_sub(learned_days),
                strings,
            });
        }
        Ok(result)
    }

    fn strings_for(connection: &Connection, inverter_id: &str) -> AppResult<Vec<StringReading>> {
        let mut statement = connection.prepare(
            "SELECT s.id, s.label, s.connected, COALESCE(s.current_point_id, ''), COALESCE(s.voltage_point_id, ''),
                    COALESCE(l.current, m.current), COALESCE(l.voltage, m.voltage), COALESCE(l.sampled_at, m.sampled_at),
                    COALESCE(a.severity, CASE WHEN EXISTS(SELECT 1 FROM plant_schemas ps
                      JOIN inverters i ON i.plant_id = ps.plant_id WHERE i.id = s.inverter_id)
                      THEN 'learning' ELSE 'unconfigured' END),
                    COALESCE(a.reason, '30 geçerli gündüz örneği bekleniyor'),
                    (SELECT COUNT(DISTINCT date(sampled_at)) FROM measurements h
                      WHERE h.string_id = s.id AND h.is_valid_daylight = 1)
             FROM strings s
             LEFT JOIN latest_readings l ON l.string_id = s.id
             LEFT JOIN measurements m ON m.id = (SELECT id FROM measurements lm WHERE lm.string_id=s.id ORDER BY sampled_at DESC LIMIT 1)
             LEFT JOIN analysis_results a ON a.id = (SELECT id FROM analysis_results la WHERE la.string_id=s.id ORDER BY analyzed_at DESC LIMIT 1)
             WHERE s.inverter_id = ?1 ORDER BY s.position",
        )?;
        let rows = statement.query_map([inverter_id], |row| {
            let severity: String = row.get(8)?;
            Ok(StringReading {
                id: row.get(0)?, label: row.get(1)?, connected: row.get(2)?, current_point_id: row.get(3)?, voltage_point_id: row.get(4)?, current: row.get(5)?, voltage: row.get(6)?,
                sampled_at: row.get::<_, Option<String>>(7)?.and_then(|value| DateTime::parse_from_rfc3339(&value).ok()).map(|value| value.with_timezone(&Utc)),
                severity: parse_severity(&severity), reason: row.get(9)?, learned_days: row.get::<_, i64>(10)? as usize,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}

fn parse_severity(value: &str) -> Severity {
    match value { "normal" => Severity::Normal, "suspicious" => Severity::Suspicious, "critical" => Severity::Critical, "night" => Severity::Night, "no_data" => Severity::NoData, "disconnected" => Severity::Disconnected, "unconfigured" => Severity::Unconfigured, _ => Severity::Learning }
}

fn parse_timezone_source(value: &str) -> TimezoneSource {
    match value { "coordinates" => TimezoneSource::Coordinates, "online_lookup" => TimezoneSource::OnlineLookup, "turkey_default" => TimezoneSource::TurkeyDefault, _ => TimezoneSource::Unknown }
}

fn parse_cloud_status(value: &str) -> CloudStatus {
    match value {
        "normal" => CloudStatus::Normal,
        "alarm" => CloudStatus::Alarm,
        "fault" => CloudStatus::Fault,
        "offline" => CloudStatus::Offline,
        "commissioning" => CloudStatus::Commissioning,
        _ => CloudStatus::Unknown,
    }
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

fn ensure_column(connection: &Connection, table: &str, column: &str, definition: &str) -> AppResult<()> {
    let mut statement = connection.prepare(&format!("PRAGMA table_info({table})"))?;
    let columns = statement.query_map([], |row| row.get::<_, String>(1))?;
    for existing in columns {
        if existing? == column {
            return Ok(());
        }
    }
    connection.execute_batch(&format!("ALTER TABLE {table} ADD COLUMN {column} {definition}"))?;
    Ok(())
}
