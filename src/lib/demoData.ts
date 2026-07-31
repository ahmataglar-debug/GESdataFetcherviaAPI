import type { DashboardSnapshot } from "../domain/models";

const sampledAt = new Date().toISOString();

export const demoSnapshot: DashboardSnapshot = {
  connectionState: "not_configured",
  lastSyncAt: sampledAt,
  nextSyncAt: null,
  warning: "Önizleme verileri gösteriliyor. OpenAPI bağlantısı henüz kurulmadı.",
  plants: [
    {
      id: "demo-plant",
      name: "Ege Güneş Santrali",
      status: "learning",
      powerKw: 448.7,
      inverterCount: 2,
      stringCount: 8,
      alertCount: 1,
      schemaConfigured: true,
      timezone: "Europe/Istanbul",
      timezoneSource: "coordinates",
      latitude: 38.42,
      longitude: 27.14,
      inverters: [
        {
          id: "inv-01",
          name: "Inverter 01",
          model: "SG125HX",
          serialNumber: "DEMO-001",
          status: "suspicious",
          powerKw: 112.4,
          strings: [
            { id: "s-01", label: "String 01", current: 9.42, voltage: 680.5, connected: true, currentPointId: "13001", voltagePointId: "13002", severity: "normal", reason: "Değerler beklenen aralıkta", learnedDays: 30, sampledAt },
            { id: "s-02", label: "String 02", current: 6.12, voltage: 675.2, connected: true, currentPointId: "13003", voltagePointId: "13004", severity: "suspicious", reason: "Akım, geçmiş ve kardeş stringlere göre düşük", learnedDays: 30, sampledAt },
            { id: "s-03", label: "String 03", current: 9.18, voltage: 678.9, connected: true, currentPointId: "13005", voltagePointId: "13006", severity: "normal", reason: "Değerler beklenen aralıkta", learnedDays: 30, sampledAt },
            { id: "s-04", label: "String 04", current: 0, voltage: 0, connected: false, currentPointId: "13007", voltagePointId: "13008", severity: "disconnected", reason: "Şemada bağlantısız; alarm değerlendirmesine alınmaz", learnedDays: 0, sampledAt }
          ]
        },
        {
          id: "inv-02",
          name: "Inverter 02",
          model: "SG125HX",
          serialNumber: "DEMO-002",
          status: "learning",
          powerKw: 109.8,
          strings: [
            { id: "s-05", label: "String 01", current: 9.11, voltage: 676.3, connected: true, currentPointId: "13001", voltagePointId: "13002", severity: "learning", reason: "30 geçerli günün 18'i toplandı", learnedDays: 18, sampledAt },
            { id: "s-06", label: "String 02", current: 8.97, voltage: 674.8, connected: true, currentPointId: "13003", voltagePointId: "13004", severity: "learning", reason: "30 geçerli günün 18'i toplandı", learnedDays: 18, sampledAt },
            { id: "s-07", label: "String 03", current: 9.04, voltage: 677.1, connected: true, currentPointId: "13005", voltagePointId: "13006", severity: "learning", reason: "30 geçerli günün 18'i toplandı", learnedDays: 18, sampledAt },
            { id: "s-08", label: "String 04", current: 9.08, voltage: 675.7, connected: true, currentPointId: "13007", voltagePointId: "13008", severity: "learning", reason: "30 geçerli günün 18'i toplandı", learnedDays: 18, sampledAt }
          ]
        }
      ]
    }
  ]
};
