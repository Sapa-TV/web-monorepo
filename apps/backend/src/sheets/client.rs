use std::sync::Mutex;
use std::time::{Duration, Instant};

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::header::CONTENT_TYPE;
use serde::Deserialize;

use crate::error::SheetsError;

const SHEETS_SCOPE: &str = "https://www.googleapis.com/auth/spreadsheets";
const DEFAULT_TOKEN_URI: &str = "https://oauth2.googleapis.com/token";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
const API_BASE: &str = "https://sheets.googleapis.com/v4/spreadsheets";
const TOKEN_REFRESH_MARGIN: Duration = Duration::from_secs(60);

#[derive(Deserialize)]
struct ServiceAccountKeyFile {
    client_email: String,
    private_key: String,
    #[serde(default = "default_token_uri")]
    token_uri: String,
}

fn default_token_uri() -> String {
    DEFAULT_TOKEN_URI.to_string()
}

struct CachedToken {
    value: String,
    expires_at: Instant,
}

#[non_exhaustive]
pub struct GoogleSheetsClient {
    http: reqwest::Client,
    client_email: String,
    token_uri: String,
    jwt_key: EncodingKey,
    cached_token: Mutex<Option<CachedToken>>,
}

impl GoogleSheetsClient {
    pub fn from_key_json(json: &str) -> Result<Self, SheetsError> {
        let key_file: ServiceAccountKeyFile = serde_json::from_str(json)
            .map_err(|e| SheetsError::Auth(format!("invalid service account key: {e}")))?;
        let jwt_key = EncodingKey::from_rsa_pem(key_file.private_key.as_bytes())
            .map_err(|e| SheetsError::Auth(format!("invalid rsa key: {e}")))?;
        Ok(Self {
            http: reqwest::Client::new(),
            client_email: key_file.client_email,
            token_uri: key_file.token_uri,
            jwt_key,
            cached_token: Mutex::new(None),
        })
    }

    pub fn from_key_base64(encoded: &str) -> Result<Self, SheetsError> {
        let json = STANDARD
            .decode(encoded.trim())
            .map_err(|e| SheetsError::Auth(format!("invalid base64 key: {e}")))?;
        let json = String::from_utf8(json)
            .map_err(|e| SheetsError::Auth(format!("key is not utf-8 json: {e}")))?;
        Self::from_key_json(&json)
    }

    async fn access_token(&self) -> Result<String, SheetsError> {
        {
            let cached = self.cached_token.lock().expect("token cache poisoned");
            if let Some(token) = cached.as_ref()
                && token.expires_at > Instant::now() + TOKEN_REFRESH_MARGIN
            {
                return Ok(token.value.clone());
            }
        }

        #[derive(serde::Serialize)]
        struct Claims<'a> {
            iss: &'a str,
            scope: &'a str,
            aud: &'a str,
            iat: u64,
            exp: u64,
        }

        #[derive(Deserialize)]
        struct TokenResponse {
            access_token: String,
            expires_in: u64,
        }

        let now = chrono::Utc::now().timestamp() as u64;
        let claims = Claims {
            iss: &self.client_email,
            scope: SHEETS_SCOPE,
            aud: &self.token_uri,
            iat: now,
            exp: now + 3600,
        };
        let assertion =
            jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &self.jwt_key)
                .map_err(|e| SheetsError::Auth(format!("jwt encode failed: {e}")))?;

        let form = url::form_urlencoded::Serializer::new(String::new())
            .append_pair("grant_type", "urn:ietf:params:oauth:grant-type:jwt-bearer")
            .append_pair("assertion", &assertion)
            .finish();
        let response = self
            .http
            .post(TOKEN_URL)
            .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
            .body(form)
            .send()
            .await
            .map_err(|e| SheetsError::Auth(format!("token request failed: {e}")))?;
        let token: TokenResponse = response
            .error_for_status()
            .map_err(|e| SheetsError::Auth(format!("token exchange rejected: {e}")))?
            .json()
            .await
            .map_err(|e| SheetsError::Auth(format!("invalid token response: {e}")))?;

        let mut cached = self.cached_token.lock().expect("token cache poisoned");
        *cached = Some(CachedToken {
            value: token.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(token.expires_in),
        });
        Ok(token.access_token)
    }

    fn values_url(spreadsheet_id: &str, range: &str) -> String {
        let encoded: String =
            percent_encoding::utf8_percent_encode(range, percent_encoding::NON_ALPHANUMERIC)
                .collect();
        format!("{API_BASE}/{spreadsheet_id}/values/{encoded}")
    }

    pub async fn get_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
    ) -> Result<Vec<Vec<String>>, SheetsError> {
        #[derive(Deserialize)]
        struct ValuesResponse {
            #[serde(default)]
            values: Vec<Vec<String>>,
        }

        let token = self.access_token().await?;
        let response = self
            .http
            .get(Self::values_url(spreadsheet_id, range))
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| SheetsError::Api(format!("values get failed: {e}")))?;
        let body: ValuesResponse = response
            .error_for_status()
            .map_err(|e| SheetsError::Api(format!("values get rejected: {e}")))?
            .json()
            .await
            .map_err(|e| SheetsError::Api(format!("invalid values response: {e}")))?;
        Ok(body.values)
    }

    pub async fn clear_values(&self, spreadsheet_id: &str, range: &str) -> Result<(), SheetsError> {
        let token = self.access_token().await?;
        let url = format!("{}:clear", Self::values_url(spreadsheet_id, range));
        let response = self
            .http
            .post(url)
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| SheetsError::Api(format!("values clear failed: {e}")))?;
        response
            .error_for_status()
            .map_err(|e| SheetsError::Api(format!("values clear rejected: {e}")))?;
        Ok(())
    }

    pub async fn update_values(
        &self,
        spreadsheet_id: &str,
        range: &str,
        values: Vec<Vec<String>>,
    ) -> Result<(), SheetsError> {
        #[derive(serde::Serialize)]
        struct UpdateBody {
            range: String,
            values: Vec<Vec<String>>,
        }

        let token = self.access_token().await?;
        let url = format!(
            "{}?valueInputOption=RAW",
            Self::values_url(spreadsheet_id, range)
        );
        let response = self
            .http
            .put(url)
            .bearer_auth(token)
            .json(&UpdateBody {
                range: range.to_string(),
                values,
            })
            .send()
            .await
            .map_err(|e| SheetsError::Api(format!("values update failed: {e}")))?;
        response
            .error_for_status()
            .map_err(|e| SheetsError::Api(format!("values update rejected: {e}")))?;
        Ok(())
    }
}
