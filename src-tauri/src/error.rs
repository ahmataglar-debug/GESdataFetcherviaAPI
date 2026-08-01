#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Yerel veritabanı hatası: {0}")]
    Database(#[from] rusqlite::Error),
    #[error("OpenAPI bağlantı hatası: {0}")]
    Http(#[from] reqwest::Error),
    #[error("Kimlik bilgisi kasası hatası: {0}")]
    Keyring(#[from] keyring::Error),
    #[error("Geçersiz URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("G/Ç hatası: {0}")]
    Io(#[from] std::io::Error),
    #[error("Geçersiz veya eksik yapılandırma: {0}")]
    Configuration(String),
    #[error("iSolarCloud API hatası: {0}")]
    Api(String),
    #[error("OAuth yetkilendirmesi zaman aşımına uğradı")]
    OAuthTimeout,
    #[error("OAuth callback sunucusu başlatılamadı: {0}")]
    OAuthCallback(String),
}

impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

pub type AppResult<T> = Result<T, AppError>;
