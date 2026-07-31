import type { InverterSummary, PlantSummary } from "../../domain/models";
import { StatusBadge } from "../../ui/StatusBadge";

interface InverterDetailProps {
  plant: PlantSummary;
  inverter: InverterSummary;
  onBack: () => void;
}

export function InverterDetail({ plant, inverter, onBack }: InverterDetailProps) {
  return (
    <>
      <button className="back-button" onClick={onBack}>← {plant.name} inverterleri</button>
      <section className="detail-hero">
        <div><span className="eyebrow">{inverter.model} · {inverter.serialNumber}</span><h2>{inverter.name}</h2><p>String bazında doğru akım ve gerilim izleme</p></div>
        <div className="hero-power"><span>TOPLAM GÜÇ</span><strong>{inverter.powerKw?.toFixed(1) ?? "—"}<small> kW</small></strong></div>
      </section>
      <section className="string-grid">
        {inverter.strings.map((item) => (
          <article className={`string-card ${item.severity}`} key={item.id}>
            <div className="card-title-row">
              <div><span className="eyebrow">DC GİRİŞİ</span><h3>{item.label}</h3></div>
              <StatusBadge severity={item.severity} />
            </div>
            <div className="string-values">
              <div><span>AKIM</span><strong>{item.current?.toFixed(2) ?? "—"}<small> A</small></strong></div>
              <div><span>GERİLİM</span><strong>{item.voltage?.toFixed(1) ?? "—"}<small> V</small></strong></div>
            </div>
            <div className="reason">{item.reason}</div>
            <div className="learning-progress">
              <div><span>Geçerli geçmiş</span><span>{Math.min(item.learnedDays, 30)} / 30 gün</span></div>
              <progress max={30} value={Math.min(item.learnedDays, 30)} />
            </div>
          </article>
        ))}
      </section>
    </>
  );
}

