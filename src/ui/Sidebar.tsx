export type Page = "plants" | "inverters" | "inverter-detail" | "schema" | "connection";

interface SidebarProps {
  page: Page;
  onNavigate: (page: Page) => void;
}

export function Sidebar({ page, onNavigate }: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">C</div>
        <div>
          <strong>Comparison</strong>
          <span>PV string intelligence</span>
        </div>
      </div>
      <nav className="side-nav" aria-label="Ana menü">
        <button className={page === "plants" || page === "inverters" || page === "inverter-detail" ? "active" : ""} onClick={() => onNavigate("plants")}>
          <span>◫</span> Güneş santralleri
        </button>
        <button className={page === "schema" ? "active" : ""} onClick={() => onNavigate("schema")}>
          <span>⌘</span> GES şemaları
        </button>
        <button className={page === "connection" ? "active" : ""} onClick={() => onNavigate("connection")}>
          <span>↗</span> OpenAPI bağlantısı
        </button>
      </nav>
      <div className="sidebar-note">
        <span className="eyebrow">ANALİZ POLİTİKASI</span>
        <strong>30 geçerli gündüz</strong>
        <p>Sarı anomali modeli bir aylık karşılaştırma verisiyle etkinleşir.</p>
      </div>
    </aside>
  );
}

