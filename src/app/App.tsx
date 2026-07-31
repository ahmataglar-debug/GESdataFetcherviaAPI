import { useEffect, useMemo, useState } from "react";
import type {
  ApiConfiguration,
  DashboardSnapshot,
  InverterSummary,
  PlantSchemaUpdate,
  PlantSummary
} from "../domain/models";
import {
  beginOAuth,
  getDashboardSnapshot,
  saveApiConfiguration,
  savePlantSchema,
  syncNow
} from "../lib/tauri";
import { ApiSetup } from "../features/auth/ApiSetup";
import { InverterDetail } from "../features/inverters/InverterDetail";
import { InverterGrid } from "../features/inverters/InverterGrid";
import { PlantGrid } from "../features/plants/PlantGrid";
import { SchemaEditor } from "../features/schema/SchemaEditor";
import { Sidebar, type Page } from "../ui/Sidebar";
import { Topbar } from "../ui/Topbar";

export function App() {
  const [snapshot, setSnapshot] = useState<DashboardSnapshot | null>(null);
  const [page, setPage] = useState<Page>("plants");
  const [selectedPlantId, setSelectedPlantId] = useState<string | null>(null);
  const [selectedInverterId, setSelectedInverterId] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const load = async () => {
    try {
      setError(null);
      setSnapshot(await getDashboardSnapshot());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    }
  };

  useEffect(() => {
    void load();
  }, []);

  const selectedPlant = useMemo(
    () => snapshot?.plants.find((plant) => plant.id === selectedPlantId) ?? null,
    [snapshot, selectedPlantId]
  );

  const selectedInverter = useMemo(
    () => selectedPlant?.inverters.find((inverter) => inverter.id === selectedInverterId) ?? null,
    [selectedPlant, selectedInverterId]
  );

  const openPlant = (plant: PlantSummary) => {
    setSelectedPlantId(plant.id);
    setSelectedInverterId(null);
    setPage("inverters");
  };

  const openInverter = (inverter: InverterSummary) => {
    setSelectedInverterId(inverter.id);
    setPage("inverter-detail");
  };

  const handleSync = async () => {
    setBusy(true);
    setError(null);
    try {
      setSnapshot(await syncNow());
    } catch (cause) {
      setError(cause instanceof Error ? cause.message : String(cause));
    } finally {
      setBusy(false);
    }
  };

  const handleApiSave = async (config: ApiConfiguration) => {
    setBusy(true);
    try {
      await saveApiConfiguration(config);
      await load();
    } finally {
      setBusy(false);
    }
  };

  const handleSchemaSave = async (schema: PlantSchemaUpdate) => {
    setBusy(true);
    try {
      await savePlantSchema(schema);
      await load();
    } finally {
      setBusy(false);
    }
  };

  const title =
    page === "plants"
      ? "GES Genel Görünüm"
      : page === "inverters"
        ? `${selectedPlant?.name ?? "GES"} · İnverterler`
        : page === "inverter-detail"
          ? `${selectedInverter?.name ?? "İnverter"} · String Detayı`
          : page === "schema"
            ? "GES Şeması"
            : "OpenAPI Bağlantısı";

  return (
    <div className="app-shell">
      <Sidebar page={page} onNavigate={setPage} />
      <div className="app-main">
        <Topbar
          title={title}
          connectionState={snapshot?.connectionState ?? "not_configured"}
          busy={busy}
          onSync={() => void handleSync()}
        />
        <main className="content">
          {error && <div className="banner banner-error">{error}</div>}
          {snapshot?.warning && <div className="banner banner-warning">{snapshot.warning}</div>}
          {!snapshot ? (
            <div className="empty-state">Yerel çalışma alanı hazırlanıyor…</div>
          ) : page === "plants" ? (
            <PlantGrid snapshot={snapshot} onOpenPlant={openPlant} />
          ) : page === "inverters" && selectedPlant ? (
            <InverterGrid plant={selectedPlant} onBack={() => setPage("plants")} onOpenInverter={openInverter} />
          ) : page === "inverter-detail" && selectedPlant && selectedInverter ? (
            <InverterDetail plant={selectedPlant} inverter={selectedInverter} onBack={() => setPage("inverters")} />
          ) : page === "schema" ? (
            <SchemaEditor plants={snapshot.plants} busy={busy} onSave={handleSchemaSave} />
          ) : page === "connection" ? (
            <ApiSetup
              state={snapshot.connectionState}
              busy={busy}
              onSave={handleApiSave}
              onAuthorize={async () => {
                await beginOAuth();
                await load();
              }}
            />
          ) : (
            <div className="empty-state">
              <h2>Önce bir GES seçin</h2>
              <button className="button primary" onClick={() => setPage("plants")}>GES listesine dön</button>
            </div>
          )}
        </main>
      </div>
    </div>
  );
}
