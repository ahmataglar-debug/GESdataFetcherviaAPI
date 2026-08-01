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

async fn receive_callback(listener: TcpListener, timeout: Duration) -> AppResult<String> {
    let accepted = tokio::time::timeout(timeout, listener.accept()).await.map_err(|_| AppError::OAuthTimeout)?;
    let (mut stream, _) = accepted?;
    let mut buffer = vec![0_u8; 8192];
    let read = stream.read(&mut buffer).await?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let target = request.lines().next().and_then(|line| line.split_whitespace().nth(1)).ok_or_else(|| AppError::Api("OAuth callback isteği okunamadı".into()))?;
    let callback = Url::parse(&format!("http://127.0.0.1{target}"))?;
    let code = callback.query_pairs().find(|(key, _)| key == "code").map(|(_, value)| value.into_owned());

    let (title, detail, color) = if code.is_some() {
        ("Yetkilendirme kodu alındı.", "Token doğrulaması için Comparison App'e dönebilirsiniz.", "#39d98a")
    } else {
        ("Yetkilendirme tamamlanamadı.", "Callback içinde authorization code bulunamadı. Comparison App'e dönüp tekrar deneyin.", "#ff7068")
    };
    let html = format!("<!doctype html><meta charset=utf-8><title>Comparison App</title><style>body{{font:16px system-ui;background:#0b1326;color:#dae2fd;display:grid;place-items:center;height:100vh;margin:0}}div{{padding:32px;border:1px solid #404751;border-radius:14px;background:#171f33}}b{{color:{color}}}</style><div><b>{title}</b><p>{detail}</p></div>");
    let response = format!("HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", html.len(), html);
    stream.write_all(response.as_bytes()).await?;
    stream.shutdown().await?;

    code.ok_or_else(|| AppError::Api("iSolarCloud callback'inde authorization code bulunamadı".into()))
}

impl OAuthService {
    pub fn authorization_url(config: &StoredApiConfiguration) -> String {
        let (site, cloud_id) = config.region.authorization_site();
        let redirect = url::form_urlencoded::byte_serialize(config.redirect_uri.as_bytes()).collect::<String>();
        let application_id = url::form_urlencoded::byte_serialize(config.application_id.as_bytes()).collect::<String>();
        format!("{site}/#/authorized-app?cloudId={cloud_id}&applicationId={application_id}&redirectUrl={redirect}")
    }

    pub async fn authorize(config: StoredApiConfiguration) -> AppResult<()> {
        let redirect = Url::parse(&config.redirect_uri)?;
        if redirect.scheme() != "http" || !matches!(redirect.host_str(), Some("127.0.0.1") | Some("localhost")) {
            return Err(AppError::Configuration("Masaüstü OAuth callback'i localhost HTTP adresi olmalıdır".into()));
        }
        let port = redirect.port_or_known_default().ok_or_else(|| AppError::Configuration("OAuth callback portu eksik".into()))?;
        let listener = TcpListener::bind(("127.0.0.1", port)).await.map_err(|error| {
            AppError::OAuthCallback(format!("127.0.0.1:{port} dinlenemiyor ({error})"))
        })?;
        open::that(Self::authorization_url(&config))?;

        let code = receive_callback(listener, Duration::from_secs(900)).await?;

        let client = OpenApiClient::from_secure_configuration(config)?;
        let tokens = client.exchange_code(&code).await?;
        SecretStore::save_tokens(&tokens)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use crate::domain::{ApiRegion, StoredApiConfiguration};

    use super::{receive_callback, OAuthService};

    #[test]
    fn portal_application_id_is_used_for_oauth() {
        let config = StoredApiConfiguration {
            app_key: "APP KEY/1".into(),
            application_id: "5843".into(),
            region: ApiRegion::Europe,
            redirect_uri: "http://127.0.0.1:42831/oauth/callback".into(),
        };

        let url = OAuthService::authorization_url(&config);
        assert!(url.contains("applicationId=5843"), "{url}");
        assert!(!url.contains("applicationId=APP"), "{url}");
        assert!(url.contains("cloudId=3"));
    }

    #[test]
    fn callback_server_accepts_the_real_http_redirect() {
        tauri::async_runtime::block_on(async {
            let listener = TcpListener::bind(("127.0.0.1", 0)).await.unwrap();
            let address = listener.local_addr().unwrap();
            let callback = tauri::async_runtime::spawn(async move {
                receive_callback(listener, Duration::from_secs(2)).await
            });

            let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
            client.write_all(b"GET /oauth/callback?code=test-code HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n").await.unwrap();
            let mut response = String::new();
            client.read_to_string(&mut response).await.unwrap();

            assert!(response.starts_with("HTTP/1.1 200 OK"), "{response}");
            assert!(response.contains("Yetkilendirme kodu alındı"), "{response}");
            assert_eq!(callback.await.unwrap().unwrap(), "test-code");
        });
    }
}
