use std::time::Duration;

use tokio::{io::{AsyncReadExt, AsyncWriteExt}, net::TcpListener};
use url::Url;

use crate::{
    domain::StoredApiConfiguration,
    error::{AppError, AppResult},
    secrets::SecretStore,
};

use super::OpenApiClient;

pub struct OAuthService;

impl OAuthService {
    pub fn authorization_url(config: &StoredApiConfiguration) -> String {
        let (site, cloud_id) = config.region.authorization_site();
        let redirect = url::form_urlencoded::byte_serialize(config.redirect_uri.as_bytes()).collect::<String>();
        format!("{site}/#/authorized-app?cloudId={cloud_id}&applicationId={}&redirectUrl={redirect}", config.application_id)
    }

    pub async fn authorize(config: StoredApiConfiguration) -> AppResult<()> {
        let redirect = Url::parse(&config.redirect_uri)?;
        if redirect.scheme() != "http" || !matches!(redirect.host_str(), Some("127.0.0.1") | Some("localhost")) {
            return Err(AppError::Configuration("Masaüstü OAuth callback'i localhost HTTP adresi olmalıdır".into()));
        }
        let port = redirect.port_or_known_default().ok_or_else(|| AppError::Configuration("OAuth callback portu eksik".into()))?;
        let listener = TcpListener::bind(("127.0.0.1", port)).await?;
        open::that(Self::authorization_url(&config))?;

        let accepted = tokio::time::timeout(Duration::from_secs(300), listener.accept()).await.map_err(|_| AppError::OAuthTimeout)?;
        let (mut stream, _) = accepted?;
        let mut buffer = vec![0_u8; 8192];
        let read = stream.read(&mut buffer).await?;
        let request = String::from_utf8_lossy(&buffer[..read]);
        let target = request.lines().next().and_then(|line| line.split_whitespace().nth(1)).ok_or_else(|| AppError::Api("OAuth callback isteği okunamadı".into()))?;
        let callback = Url::parse(&format!("http://127.0.0.1:{port}{target}"))?;
        let code = callback.query_pairs().find(|(key, _)| key == "code").map(|(_, value)| value.into_owned()).ok_or_else(|| AppError::Api("iSolarCloud callback'inde authorization code bulunamadı".into()))?;

        let client = OpenApiClient::from_secure_configuration(config)?;
        let tokens = client.exchange_code(&code).await?;
        SecretStore::save_tokens(&tokens)?;
        let html = "<!doctype html><meta charset=utf-8><title>Comparison App</title><style>body{font:16px system-ui;background:#0b1326;color:#dae2fd;display:grid;place-items:center;height:100vh;margin:0}div{padding:32px;border:1px solid #404751;border-radius:14px;background:#171f33}b{color:#39d98a}</style><div><b>Yetkilendirme tamamlandı.</b><p>Bu pencereyi kapatıp Comparison App'e dönebilirsiniz.</p></div>";
        let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", html.len(), html);
        stream.write_all(response.as_bytes()).await?;
        Ok(())
    }
}
