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
  const [currentThreshold, setCurrentThreshold] = useState(0.15);
  const [voltageThreshold, setVoltageThreshold] = useState(10);

  useEffect(() => {
    if (!plant) return;
    setConnected(Object.fromEntries(plant.inverters.flatMap((inverter) => inverter.strings.map((item) => [item.id, item.connected]))));
  }, [plant]);

  if (!plant) return <div className="empty-state">OpenAPI’den GES geldikten sonra şema hazırlanabilir.</div>;

  const save = async () => {
    await onSave({
      plantId: plant.id,
      currentZeroThreshold: currentThreshold,
      voltageZeroThreshold: voltageThreshold,
      strings: Object.entries(connected).map(([id, isConnected]) => ({ id, connected: isConnected }))
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
          <div className="schema-inverter"><strong>{inverter.name}</strong><span>{inverter.model} · {inverter.serialNumber}</span></div>
          <table className="schema-table">
            <thead><tr><th>String</th><th>Son akım</th><th>Son gerilim</th><th>Fiziksel bağlantı</th></tr></thead>
            <tbody>{inverter.strings.map((item) => (
              <tr key={item.id}><td>{item.label}</td><td>{item.current ?? "—"} A</td><td>{item.voltage ?? "—"} V</td><td><label className="switch-label"><input type="checkbox" checked={connected[item.id] ?? false} onChange={(event) => setConnected({ ...connected, [item.id]: event.target.checked })} /><span>{connected[item.id] ? "Bağlı" : "Boş / bağlantısız"}</span></label></td></tr>
            ))}</tbody>
          </table>
        </div>
      ))}
      <button className="button primary schema-save" disabled={busy} onClick={() => void save()}>GES şemasını kaydet</button>
    </section>
  );
}
