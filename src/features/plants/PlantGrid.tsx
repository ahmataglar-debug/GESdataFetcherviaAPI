import type { DashboardSnapshot, PlantSummary } from "../../domain/models";
import { StatusBadge } from "../../ui/StatusBadge";
import { CloudStatusBadge } from "../../ui/CloudStatusBadge";

interface PlantGridProps {
  snapshot: DashboardSnapshot;
  onOpenPlant: (plant: PlantSummary) => void;
}

export function PlantGrid({ snapshot, onOpenPlant }: PlantGridProps) {
  const inverterCount = snapshot.plants.reduce((sum, plant) => sum + plant.inverterCount, 0);
  const alertCount = snapshot.plants.reduce((sum, plant) => sum + plant.alertCount, 0);
  const cloudAlarmCount = snapshot.plants.reduce((sum, plant) => sum + plant.cloudAlarmCount, 0);

  return (
    <>
      <section className="metric-row">
        <article><span>GES</span><strong>{snapshot.plants.length}</strong><small>erişilebilir santral</small></article>
        <article><span>İNVERTER</span><strong>{inverterCount}</strong><small>OpenAPI cihazı</small></article>
        <article><span>AKTİF UYARI</span><strong className={alertCount ? "amber" : "green"}>{alertCount}</strong><small>uygulama analizi</small></article>
        <article><span>ISOLARCLOUD UYARISI</span><strong className={cloudAlarmCount ? "amber" : "green"}>{cloudAlarmCount}</strong><small>API tarafından bildirildi</small></article>
        <article><span>SON SENKRON</span><strong className="metric-date">{snapshot.lastSyncAt ? new Date(snapshot.lastSyncAt).toLocaleString("tr-TR") : "—"}</strong><small>yerel saat</small></article>
      </section>
      <div className="section-heading">
        <div><span className="eyebrow">PORTFÖY</span><h2>Güneş santralleri</h2></div>
        <span>{snapshot.plants.length} kayıt</span>
      </div>
      <section className="plant-grid">
        {snapshot.plants.map((plant) => (
          <article className="plant-card" key={plant.id}>
            <div className="plant-visual">
              <div className="sun-orbit" />
              <div className="panel-lines" />
              <StatusBadge severity={plant.status} />
              <CloudStatusBadge status={plant.cloudStatus} alarmCount={plant.cloudAlarmCount} />
            </div>
            <div className="plant-body">
              <div>
                <h3>{plant.name}</h3>
                <p>{plant.timezone} · {plant.timezoneSource === "coordinates" ? "koordinattan belirlendi" : "tahmini"}</p>
              </div>
              <div className="plant-stats">
                <div><span>ÜRETİM</span><strong>{plant.powerKw?.toFixed(1) ?? "—"}<small> kW</small></strong></div>
                <div><span>İNVERTER</span><strong>{plant.inverterCount}</strong></div>
                <div><span>STRING</span><strong>{plant.stringCount ?? "—"}</strong></div>
              </div>
              <div className="analysis-countdown">
                <div><span>ANALİZ ÖĞRENMESİ</span><strong>{plant.schemaConfigured ? (plant.daysUntilAnalysis > 0 ? `${plant.daysUntilAnalysis} gündüz kaldı` : "Analiz etkin") : "Şema bekleniyor"}</strong></div>
                <progress max={30} value={plant.learnedDays} />
                <small>{plant.schemaConfigured ? `${plant.learnedDays} / 30 geçerli gündüz` : "Şema kaydedildikten sonra 30 geçerli gündüz toplanacak"}</small>
              </div>
              <div className="card-footer">
                <span className={plant.schemaConfigured ? "schema-ok" : "schema-missing"}>{plant.schemaConfigured ? "Şema hazır" : "Şema gerekli"}</span>
                <button className="text-button" onClick={() => onOpenPlant(plant)}>İnverterleri görüntüle →</button>
              </div>
            </div>
          </article>
        ))}
      </section>
    </>
  );
}
