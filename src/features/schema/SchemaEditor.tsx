import { useEffect, useMemo, useState } from "react";
import type { PlantSchemaUpdate, PlantSummary } from "../../domain/models";

interface SchemaEditorProps {
  plants: PlantSummary[];
  busy: boolean;
  onSave: (schema: PlantSchemaUpdate) => Promise<void>;
}

export function SchemaEditor({ plants, busy, onSave }: SchemaEditorProps) {
  const [plantId, setPlantId] = useState(plants[0]?.id ?? "");
  const plant = useMemo(() => plants.find((item) => item.id === plantId), [plants, plantId]);
  const [connected, setConnected] = useState<Record<string, boolean>>({});
  const [drafts, setDrafts] = useState<Record<string, Array<{ id: string; position: number; label: string; currentPointId: string; voltagePointId: string; connected: boolean }>>>({});
  const [currentThreshold, setCurrentThreshold] = useState(0.15);
  const [voltageThreshold, setVoltageThreshold] = useState(10);

  useEffect(() => {
    if (!plant) return;
    setConnected(Object.fromEntries(plant.inverters.flatMap((inverter) => inverter.strings.map((item) => [item.id, item.connected]))));
    setDrafts(Object.fromEntries(plant.inverters.map((inverter) => [inverter.id, inverter.strings.map((item, index) => ({
      id: item.id,
      position: index + 1,
      label: item.label,
      currentPointId: item.currentPointId,
      voltagePointId: item.voltagePointId,
      connected: item.connected
    }))])));
  }, [plant]);

  if (!plant) return <div className="empty-state">OpenAPI’den GES geldikten sonra şema hazırlanabilir.</div>;

  const save = async () => {
    await onSave({
      plantId: plant.id,
      currentZeroThreshold: currentThreshold,
      voltageZeroThreshold: voltageThreshold,
      strings: plant.inverters.flatMap((inverter) => (drafts[inverter.id] ?? []).map((item, index) => ({
        id: item.id || `${inverter.id}:string:${index + 1}`,
        inverterId: inverter.id,
        position: index + 1,
        label: item.label || `String ${String(index + 1).padStart(2, "0")}`,
        currentPointId: item.currentPointId,
        voltagePointId: item.voltagePointId,
        connected: connected[item.id] ?? item.connected
      })))
    });
  };

  return (
    <section className="schema-layout">
      <div className="schema-toolbar">
        <div><span className="eyebrow">KRİTİK UYARI KAYNAĞI</span><h2>Bağlantı şeması</h2><p>Yalnızca bağlı işaretlenen stringler sıfıra yakınlık alarmı üretebilir.</p></div>
        <select value={plantId} onChange={(event) => setPlantId(event.target.value)}>{plants.map((item) => <option key={item.id} value={item.id}>{item.name}</option>)}</select>
      </div>
      <div className="threshold-row">
        <label>Akım sıfır eşiği <span><input type="number" min="0" step="0.01" value={currentThreshold} onChange={(event) => setCurrentThreshold(Number(event.target.value))} /> A</span></label>
        <label>Gerilim sıfır eşiği <span><input type="number" min="0" step="1" value={voltageThreshold} onChange={(event) => setVoltageThreshold(Number(event.target.value))} /> V</span></label>
        <div className="timezone-box"><span>OTOMATİK SAAT DİLİMİ</span><strong>{plant.timezone}</strong><small>{plant.timezoneSource === "coordinates" ? "GES koordinatından" : "tahmini"}</small></div>
      </div>
      {plant.inverters.map((inverter) => (
        <div className="schema-table-wrap" key={inverter.id}>
          <div className="schema-inverter"><strong>{inverter.name}</strong><span>{inverter.model} · {inverter.serialNumber}</span><button className="text-button schema-add" onClick={() => setDrafts({ ...drafts, [inverter.id]: [...(drafts[inverter.id] ?? []), { id: `${inverter.id}:string:${(drafts[inverter.id]?.length ?? 0) + 1}`, position: (drafts[inverter.id]?.length ?? 0) + 1, label: `String ${String((drafts[inverter.id]?.length ?? 0) + 1).padStart(2, "0")}`, currentPointId: "", voltagePointId: "", connected: true }] })}>+ String ekle</button></div>
          <table className="schema-table">
            <thead><tr><th>String</th><th>Akım point ID</th><th>Gerilim point ID</th><th>Fiziksel bağlantı</th></tr></thead>
            <tbody>{(drafts[inverter.id] ?? []).map((item, index) => (
              <tr key={item.id}><td><input value={item.label} onChange={(event) => setDrafts({ ...drafts, [inverter.id]: (drafts[inverter.id] ?? []).map((row, rowIndex) => rowIndex === index ? { ...row, label: event.target.value } : row) })} /></td><td><input inputMode="numeric" placeholder="örn. 13001" value={item.currentPointId} onChange={(event) => setDrafts({ ...drafts, [inverter.id]: (drafts[inverter.id] ?? []).map((row, rowIndex) => rowIndex === index ? { ...row, currentPointId: event.target.value.replace(/\D/g, "") } : row) })} /></td><td><input inputMode="numeric" placeholder="örn. 13002" value={item.voltagePointId} onChange={(event) => setDrafts({ ...drafts, [inverter.id]: (drafts[inverter.id] ?? []).map((row, rowIndex) => rowIndex === index ? { ...row, voltagePointId: event.target.value.replace(/\D/g, "") } : row) })} /></td><td><label className="switch-label"><input type="checkbox" checked={connected[item.id] ?? item.connected} onChange={(event) => setConnected({ ...connected, [item.id]: event.target.checked })} /><span>{connected[item.id] ?? item.connected ? "Bağlı" : "Boş / bağlantısız"}</span></label></td></tr>
            ))}</tbody>
          </table>
          {(drafts[inverter.id]?.length ?? 0) === 0 && <div className="schema-empty">Bu inverter için string eşlemesi eklenmedi.</div>}
        </div>
      ))}
      <button className="button primary schema-save" disabled={busy} onClick={() => void save()}>GES şemasını kaydet</button>
    </section>
  );
}
