CREATE TABLE IF NOT EXISTS settings (
  key TEXT PRIMARY KEY,
  value TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS plants (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  latitude REAL,
  longitude REAL,
  timezone TEXT NOT NULL DEFAULT 'Europe/Istanbul',
  timezone_source TEXT NOT NULL DEFAULT 'turkey_default',
  power_kw REAL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS inverters (
  id TEXT PRIMARY KEY,
  plant_id TEXT NOT NULL REFERENCES plants(id) ON DELETE CASCADE,
  name TEXT NOT NULL,
  model TEXT NOT NULL DEFAULT '',
  serial_number TEXT NOT NULL DEFAULT '',
  power_kw REAL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS strings (
  id TEXT PRIMARY KEY,
  inverter_id TEXT NOT NULL REFERENCES inverters(id) ON DELETE CASCADE,
  position INTEGER NOT NULL,
  label TEXT NOT NULL,
  current_point_id TEXT,
  voltage_point_id TEXT,
  connected INTEGER NOT NULL DEFAULT 1,
  UNIQUE(inverter_id, position)
);

CREATE TABLE IF NOT EXISTS plant_schemas (
  plant_id TEXT PRIMARY KEY REFERENCES plants(id) ON DELETE CASCADE,
  current_zero_threshold REAL NOT NULL DEFAULT 0.15,
  voltage_zero_threshold REAL NOT NULL DEFAULT 10.0,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS measurements (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  string_id TEXT NOT NULL REFERENCES strings(id) ON DELETE CASCADE,
  current REAL,
  voltage REAL,
  sampled_at TEXT NOT NULL,
  is_valid_daylight INTEGER NOT NULL,
  UNIQUE(string_id, sampled_at)
);
CREATE INDEX IF NOT EXISTS idx_measurements_string_time ON measurements(string_id, sampled_at DESC);

CREATE TABLE IF NOT EXISTS latest_readings (
  string_id TEXT PRIMARY KEY REFERENCES strings(id) ON DELETE CASCADE,
  current REAL,
  voltage REAL,
  sampled_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS analysis_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  string_id TEXT NOT NULL REFERENCES strings(id) ON DELETE CASCADE,
  measurement_id INTEGER REFERENCES measurements(id) ON DELETE CASCADE,
  severity TEXT NOT NULL,
  reason TEXT NOT NULL,
  analyzed_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS scheduler_state (
  id INTEGER PRIMARY KEY CHECK(id = 1),
  last_attempt_at TEXT,
  last_success_at TEXT,
  last_valid_daylight_at TEXT,
  last_attempt_was_night INTEGER NOT NULL DEFAULT 0,
  next_sync_at TEXT
);

CREATE TABLE IF NOT EXISTS sync_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at TEXT NOT NULL,
  completed_at TEXT,
  status TEXT NOT NULL,
  error_code TEXT,
  error_message TEXT
);
