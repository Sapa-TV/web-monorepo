use serde::Deserialize;

use crate::error::{Error, Result};
use crate::transport::Transport;
use crate::url::encode_component;

pub const AUTHORIZE_URL: &str = "https://auth.live.vkvideo.ru/app/oauth2/authorize";
pub const TOKEN_URL: &str = "https://api.live.vkvideo.ru/oauth/server/token";
pub const REVOKE_URL: &str = "https://api.live.vkvideo.ru/oauth/server/revoke";

pub const INGRESS_SCOPES: &[&str] = &["chat:message:send"];

const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

#[derive(Debug, Clone)]
#[non_exhaustive]
pub enum TokenHint {
    Access,
    Refresh,
}

impl TokenHint {
    fn as_str(&self) -> &'static str {
        match self {
            TokenHint::Access => "access_token",
            TokenHint::Refresh => "refresh_token",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_in: u64,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone)]
pub struct OAuthClient {
    client_id: String,
    client_secret: String,
}

impl OAuthClient {
    pub fn new(client_id: impl Into<String>, client_secret: impl Into<String>) -> Self {
        Self {
            client_id: client_id.into(),
            client_secret: client_secret.into(),
        }
    }

    pub fn authorize_url(&self, redirect_uri: &str, scopes: &[&str], state: &str) -> String {
        let mut url = String::from(AUTHORIZE_URL);
        url.push_str("?client_id=");
        url.push_str(&encode_component(&self.client_id));
        url.push_str("&redirect_uri=");
        url.push_str(&encode_component(redirect_uri));
        url.push_str("&response_type=code");
        if !scopes.is_empty() {
            url.push_str("&scope=");
            url.push_str(&encode_component(&scopes.join(",")));
        }
        if !state.is_empty() {
            url.push_str("&state=");
            url.push_str(&encode_component(state));
        }
        url
    }

    pub async fn exchange_code<T: Transport>(
        &self,
        transport: &T,
        code: &str,
        redirect_uri: &str,
    ) -> Result<TokenResponse> {
        let form = [
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ];
        self.token(transport, &form).await
    }

    pub async fn refresh<T: Transport>(
        &self,
        transport: &T,
        refresh_token: &str,
        redirect_uri: &str,
    ) -> Result<TokenResponse> {
        let form = [
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
            ("redirect_uri", redirect_uri),
        ];
        self.token(transport, &form).await
    }

    pub async fn revoke<T: Transport>(
        &self,
        transport: &T,
        token: &str,
        hint: TokenHint,
    ) -> Result<()> {
        let form = [("token", token), ("token_type_hint", hint.as_str())];
        transport
            .post_form(REVOKE_URL, &self.basic_auth(), &form_body(&form))
            .await?;
        Ok(())
    }

    async fn token<T: Transport>(
        &self,
        transport: &T,
        form: &[(&str, &str)],
    ) -> Result<TokenResponse> {
        let body = transport
            .post_form(TOKEN_URL, &self.basic_auth(), &form_body(form))
            .await?;
        serde_json::from_str(&body).map_err(|e| Error::Protocol(format!("token response: {e}")))
    }

    fn basic_auth(&self) -> String {
        let credentials = format!("{}:{}", self.client_id, self.client_secret);
        format!("Basic {}", base64_encode(credentials.as_bytes()))
    }
}

fn form_body(form: &[(&str, &str)]) -> String {
    form.iter()
        .map(|(k, v)| format!("{}={}", encode_component(k), encode_component(v)))
        .collect::<Vec<_>>()
        .join("&")
}

fn base64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = u32::from(chunk[0]);
        let b1 = u32::from(*chunk.get(1).unwrap_or(&0));
        let b2 = u32::from(*chunk.get(2).unwrap_or(&0));
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(BASE64_TABLE[(n >> 18) as usize & 63] as char);
        out.push(BASE64_TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            BASE64_TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            BASE64_TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
#[path = "auth.test.rs"]
mod tests;
