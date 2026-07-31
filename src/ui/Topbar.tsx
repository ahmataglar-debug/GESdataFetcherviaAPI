import type { DashboardSnapshot } from "../domain/models";

interface TopbarProps {
  title: string;
  connectionState: DashboardSnapshot["connectionState"];
  busy: boolean;
  onSync: () => void;
}

const labels: Record<DashboardSnapshot["connectionState"], string> = {
  not_configured: "API yapılandırılmadı",
  authorizing: "Yetki bekleniyor",
  connected: "OpenAPI bağlı",
  error: "Bağlantı hatası"
};

export function Topbar({ title, connectionState, busy, onSync }: TopbarProps) {
  return (
    <header className="topbar">
      <div>
        <span className="eyebrow">TEKNİK İZLEME</span>
        <h1>{title}</h1>
      </div>
      <div className="topbar-actions">
        <span className={`connection-pill ${connectionState}`}>
          <i /> {labels[connectionState]}
        </span>
        <button className="button secondary" disabled={busy} onClick={onSync}>
          {busy ? "Senkronize ediliyor…" : "Şimdi senkronize et"}
        </button>
      </div>
    </header>
  );
}

