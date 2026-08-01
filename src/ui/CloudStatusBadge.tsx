import type { CloudStatus } from "../domain/models";

const labels: Record<CloudStatus, string> = {
  normal: "iSolarCloud Normal",
  alarm: "iSolarCloud Alarm",
  fault: "iSolarCloud Arıza",
  offline: "iSolarCloud Çevrimdışı",
  commissioning: "iSolarCloud Devreye alma",
  unknown: "iSolarCloud Durumu bilinmiyor"
};

export function CloudStatusBadge({ status, alarmCount = 0 }: { status: CloudStatus; alarmCount?: number }) {
  const count = alarmCount > 0 ? ` (${alarmCount})` : "";
  return <span className={`cloud-status-badge cloud-${status}`}><i />{labels[status]}{count}</span>;
}
