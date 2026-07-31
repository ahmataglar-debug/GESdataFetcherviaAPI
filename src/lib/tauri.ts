import { invoke } from "@tauri-apps/api/core";
import type {
  ApiConfiguration,
  DashboardSnapshot,
  PlantSchemaUpdate
} from "../domain/models";
import { demoSnapshot } from "./demoData";

const isTauri = () => "__TAURI_INTERNALS__" in window;

export async function getDashboardSnapshot(): Promise<DashboardSnapshot> {
  if (!isTauri()) return demoSnapshot;
  return invoke<DashboardSnapshot>("get_dashboard_snapshot");
}

export async function saveApiConfiguration(config: ApiConfiguration): Promise<void> {
  if (!isTauri()) return;
  await invoke("save_api_configuration", { config });
}

export async function beginOAuth(): Promise<void> {
  if (!isTauri()) throw new Error("OAuth yalnızca masaüstü uygulamasında kullanılabilir.");
  await invoke("begin_oauth");
}

export async function syncNow(): Promise<DashboardSnapshot> {
  if (!isTauri()) return demoSnapshot;
  return invoke<DashboardSnapshot>("sync_now");
}

export async function savePlantSchema(schema: PlantSchemaUpdate): Promise<void> {
  if (!isTauri()) return;
  await invoke("save_plant_schema", { schema });
}

