import { useState } from "react";
import type { ApiConfiguration, DashboardSnapshot } from "../../domain/models";

interface ApiSetupProps {
  state: DashboardSnapshot["connectionState"];
  busy: boolean;
  onSave: (config: ApiConfiguration) => Promise<void>;
  onAuthorize: () => Promise<void>;
}

export function ApiSetup({ state, busy, onSave, onAuthorize }: ApiSetupProps) {
  const [config, setConfig] = useState<ApiConfiguration>({
    appKey: "",
    secretKey: "",
    region: "europe",
    redirectUri: "http://127.0.0.1:42831/oauth/callback"
  });
  const [message, setMessage] = useState<string | null>(null);

  const submit = async (event: React.FormEvent) => {
    event.preventDefault();
    setMessage(null);
    try {
      await onSave(config);
      setConfig((current) => ({ ...current, secretKey: "" }));
      setMessage("Yapılandırma kaydedildi. Secret key güvenli kasaya aktarıldı.");
    } catch (cause) {
      setMessage(cause instanceof Error ? cause.message : String(cause));
    }
  };

  return (
    <section className="setup-layout">
      <form className="setup-card" onSubmit={(event) => void submit(event)}>
        <span className="eyebrow">SUNGROW DEVELOPER PORTAL</span>
        <h2>OpenAPI yapılandırması</h2>
        <p>Buraya iSolarCloud parolanızı değil, OAuth etkin uygulamanızın yenilenmiş geliştirici anahtarlarını girin.</p>
        <label>AppKey<input value={config.appKey} onChange={(event) => setConfig({ ...config, appKey: event.target.value.trim() })} autoComplete="off" required /></label>
        <label>Secret key<input type="password" value={config.secretKey} onChange={(event) => setConfig({ ...config, secretKey: event.target.value.trim() })} autoComplete="new-password" required /></label>
        <label>Bölge
          <select value={config.region} onChange={(event) => setConfig({ ...config, region: event.target.value as ApiConfiguration["region"] })}>
            <option value="europe">Avrupa / Türkiye</option>
            <option value="international">Uluslararası</option>
            <option value="australia">Avustralya</option>
            <option value="china">Çin</option>
          </select>
        </label>
        <label>OAuth redirect URI<input value={config.redirectUri} onChange={(event) => setConfig({ ...config, redirectUri: event.target.value })} required /></label>
        {message && <div className="form-message">{message}</div>}
        <div className="form-actions">
          <button className="button primary" disabled={busy}>Ayarları güvenle kaydet</button>
          <button className="button secondary" type="button" disabled={busy || state === "not_configured"} onClick={() => void onAuthorize()}>Tarayıcıda yetkilendir</button>
        </div>
      </form>
      <aside className="security-card">
        <span className="security-icon">◇</span>
        <h3>Kimlik bilgisi sınırı</h3>
        <ul>
          <li>iSolarCloud şifresi uygulama tarafından alınmaz.</li>
          <li>Secret key ve OAuth tokenları işletim sistemi kasasında tutulur.</li>
          <li>React katmanına ham anahtar veya token gönderilmez.</li>
          <li>Paylaşılan eski anahtarı kullanmadan önce yenileyin.</li>
        </ul>
      </aside>
    </section>
  );
}
