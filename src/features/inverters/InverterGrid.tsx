import type { InverterSummary, PlantSummary } from "../../domain/models";
import { StatusBadge } from "../../ui/StatusBadge";

interface InverterGridProps {
  plant: PlantSummary;
  onBack: () => void;
  onOpenInverter: (inverter: InverterSummary) => void;
}

export function InverterGrid({ plant, onBack, onOpenInverter }: InverterGridProps) {
  return (
    <>
      <button className="back-button" onClick={onBack}>← GES listesi</button>
      <div className="section-heading">
        <div><span className="eyebrow">{plant.name}</span><h2>İnverterler</h2></div>
        <span>{plant.inverters.length} cihaz</span>
      </div>
      <section className="inverter-grid">
        {plant.inverters.map((inverter) => {
          const warnings = inverter.strings.filter((item) => item.severity === "suspicious" || item.severity === "critical").length;
          return (
            <article className="inverter-card" key={inverter.id}>
              <div className="card-title-row">
                <div className="device-icon">⌁</div>
                <div><h3>{inverter.name}</h3><p>{inverter.model} · {inverter.serialNumber}</p></div>
                <StatusBadge severity={inverter.status} />
              </div>
              <div className="inverter-data">
                <div><span>GÜÇ</span><strong>{inverter.powerKw?.toFixed(1) ?? "—"}<small> kW</small></strong></div>
                <div><span>STRING</span><strong>{inverter.strings.length}</strong></div>
                <div><span>UYARI</span><strong className={warnings ? "amber" : "green"}>{warnings}</strong></div>
              </div>
              <button className="button card-action" onClick={() => onOpenInverter(inverter)}>String değerlerini aç</button>
            </article>
          );
        })}
      </section>
    </>
  );
}

