export type Severity =
  | "normal"
  | "suspicious"
  | "critical"
  | "learning"
  | "night"
  | "no_data"
  | "disconnected"
  | "unconfigured";

export interface StringReading {
  id: string;
  label: string;
  current: number | null;
  voltage: number | null;
  connected: boolean;
  currentPointId: string;
  voltagePointId: string;
  severity: Severity;
  reason: string;
  learnedDays: number;
  sampledAt: string | null;
}

export interface InverterSummary {
  id: string;
  name: string;
  model: string;
  serialNumber: string;
  status: Severity;
  powerKw: number | null;
  strings: StringReading[];
}

export interface PlantSummary {
  id: string;
  name: string;
  status: Severity;
  powerKw: number | null;
  inverterCount: number;
  stringCount: number;
  alertCount: number;
  schemaConfigured: boolean;
  timezone: string;
  timezoneSource: "coordinates" | "online_lookup" | "turkey_default" | "unknown";
  latitude: number | null;
  longitude: number | null;
  inverters: InverterSummary[];
}

export interface DashboardSnapshot {
  connectionState: "not_configured" | "authorizing" | "connected" | "error";
  lastSyncAt: string | null;
  nextSyncAt: string | null;
  plants: PlantSummary[];
  warning?: string;
}

export interface ApiConfiguration {
  appKey: string;
  secretKey: string;
  applicationId: string;
  region: "europe" | "international" | "china" | "australia";
  redirectUri: string;
}

export interface PlantSchemaUpdate {
  plantId: string;
  currentZeroThreshold: number;
  voltageZeroThreshold: number;
  strings: Array<{
    id: string;
    inverterId: string;
    position: number;
    label: string;
    currentPointId: string;
    voltagePointId: string;
    connected: boolean;
  }>;
}
