import type { Severity } from "../domain/models";

const label: Record<Severity, string> = {
  normal: "Normal",
  suspicious: "Şüpheli",
  critical: "Kritik",
  learning: "Öğreniyor",
  night: "Gece örneği",
  no_data: "Veri yok",
  disconnected: "Bağlantısız",
  unconfigured: "Şema gerekli"
};

export function StatusBadge({ severity }: { severity: Severity }) {
  return <span className={`status-badge ${severity}`}><i />{label[severity]}</span>;
}
