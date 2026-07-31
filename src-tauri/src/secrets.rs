use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

const SERVICE: &str = "com.mtcenerji.comparisonapp";
const API_SECRET_ACCOUNT: &str = "isolarcloud-api-secret";
const OAUTH_TOKEN_ACCOUNT: &str = "isolarcloud-oauth-tokens";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
}

pub struct SecretStore;

impl SecretStore {
    fn entry(account: &str) -> AppResult<keyring::Entry> {
        Ok(keyring::Entry::new(SERVICE, account)?)
    }

    pub fn save_api_secret(secret: &str) -> AppResult<()> {
        Self::entry(API_SECRET_ACCOUNT)?.set_password(secret)?;
        Ok(())
    }

    pub fn api_secret() -> AppResult<String> {
        Self::entry(API_SECRET_ACCOUNT)?
            .get_password()
            .map_err(AppError::from)
    }

    pub fn save_tokens(tokens: &OAuthTokens) -> AppResult<()> {
        let encoded = serde_json::to_string(tokens)
            .map_err(|error| AppError::Configuration(error.to_string()))?;
        Self::entry(OAUTH_TOKEN_ACCOUNT)?.set_password(&encoded)?;
        Ok(())
    }

    pub fn tokens() -> AppResult<Option<OAuthTokens>> {
        match Self::entry(OAUTH_TOKEN_ACCOUNT)?.get_password() {
            Ok(value) => serde_json::from_str(&value)
                .map(Some)
                .map_err(|error| AppError::Configuration(error.to_string())),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }
}
