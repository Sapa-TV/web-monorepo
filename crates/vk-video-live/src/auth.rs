use serde::Deserialize;

use crate::error::{Error, Result};
use crate::transport::Transport;
use crate::url::encode_component;

pub const AUTHORIZE_URL: &str = "https://auth.live.vkvideo.ru/app/oauth2/authorize";
pub const TOKEN_URL: &str = "https://api.live.vkvideo.ru/oauth/server/token";
pub const REVOKE_URL: &str = "https://api.live.vkvideo.ru/oauth/server/revoke";

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
            .post_form(REVOKE_URL, &self.basic_auth(), &form)
            .await?;
        Ok(())
    }

    async fn token<T: Transport>(
        &self,
        transport: &T,
        form: &[(&str, &str)],
    ) -> Result<TokenResponse> {
        let body = transport
            .post_form(TOKEN_URL, &self.basic_auth(), form)
            .await?;
        serde_json::from_str(&body).map_err(|e| Error::Protocol(format!("token response: {e}")))
    }

    fn basic_auth(&self) -> String {
        let credentials = format!("{}:{}", self.client_id, self.client_secret);
        format!("Basic {}", base64_encode(credentials.as_bytes()))
    }
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
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[derive(Debug, Clone)]
    struct Record {
        url: String,
        auth: String,
        form: Vec<(String, String)>,
    }

    #[derive(Debug, Clone)]
    struct FakeTransport {
        response: Result<String>,
        records: Arc<Mutex<Vec<Record>>>,
    }

    impl FakeTransport {
        fn new(response: Result<String>) -> Self {
            Self {
                response,
                records: Arc::new(Mutex::new(Vec::new())),
            }
        }

        fn last(&self) -> Record {
            self.records.lock().unwrap().last().unwrap().clone()
        }
    }

    impl Transport for FakeTransport {
        async fn post_form(
            &self,
            url: &str,
            basic_auth: &str,
            form: &[(&str, &str)],
        ) -> Result<String> {
            self.records.lock().unwrap().push(Record {
                url: url.to_string(),
                auth: basic_auth.to_string(),
                form: form
                    .iter()
                    .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                    .collect(),
            });
            self.response.clone()
        }

        async fn get(&self, _url: &str, _bearer: &str) -> Result<String> {
            Err(Error::Protocol("unexpected GET".to_string()))
        }

        async fn post_json(&self, _url: &str, _bearer: &str, _body: &str) -> Result<String> {
            Err(Error::Protocol("unexpected POST JSON".to_string()))
        }
    }

    fn client() -> OAuthClient {
        OAuthClient::new("id123", "secret456")
    }

    #[test]
    fn base64_encode_standard_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(base64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn encode_component_keeps_unreserved_only() {
        assert_eq!(encode_component("abc-._~123"), "abc-._~123");
        assert_eq!(
            encode_component("https://sapa-tv.ru/creds-callback/twitch"),
            "https%3A%2F%2Fsapa-tv.ru%2Fcreds-callback%2Ftwitch"
        );
        assert_eq!(encode_component("a b+c"), "a%20b%2Bc");
    }

    #[test]
    fn authorize_url_encodes_query() {
        let url = client().authorize_url(
            "https://sapa-tv.ru/login-callback/twitch",
            &["chat:message:send", "channel:points"],
            "st ate/1",
        );
        assert_eq!(
            url,
            "https://auth.live.vkvideo.ru/app/oauth2/authorize\
?client_id=id123\
&redirect_uri=https%3A%2F%2Fsapa-tv.ru%2Flogin-callback%2Ftwitch\
&response_type=code\
&scope=chat%3Amessage%3Asend%2Cchannel%3Apoints\
&state=st%20ate%2F1"
        );
    }

    #[test]
    fn authorize_url_without_scope_and_state() {
        let url = client().authorize_url("https://x.example/cb", &[], "");
        assert!(url.ends_with("&response_type=code"));
        assert!(!url.contains("scope="));
        assert!(!url.contains("state="));
    }

    #[tokio::test]
    async fn exchange_code_sends_basic_auth_and_form() {
        let transport = FakeTransport::new(Ok(
            r#"{"access_token":"at","refresh_token":"rt","expires_in":86400,"token_type":"Bearer"}"#
                .to_string(),
        ));
        let token = client()
            .exchange_code(
                &transport,
                "the-code",
                "https://sapa-tv.ru/login-callback/twitch",
            )
            .await
            .unwrap();

        let record = transport.last();
        assert_eq!(record.url, TOKEN_URL);
        assert_eq!(record.auth, "Basic aWQxMjM6c2VjcmV0NDU2");
        assert_eq!(
            record.form,
            vec![
                ("grant_type".to_string(), "authorization_code".to_string()),
                ("code".to_string(), "the-code".to_string()),
                (
                    "redirect_uri".to_string(),
                    "https://sapa-tv.ru/login-callback/twitch".to_string()
                ),
            ]
        );
        assert_eq!(token.access_token, "at");
        assert_eq!(token.refresh_token, "rt");
        assert_eq!(token.expires_in, 86400);
    }

    #[tokio::test]
    async fn refresh_sends_refresh_grant() {
        let transport = FakeTransport::new(Ok(
            r#"{"access_token":"at2","refresh_token":"rt2","expires_in":60}"#.to_string(),
        ));
        let token = client()
            .refresh(
                &transport,
                "rt-old",
                "https://sapa-tv.ru/login-callback/twitch",
            )
            .await
            .unwrap();

        let record = transport.last();
        assert_eq!(
            record.form,
            vec![
                ("grant_type".to_string(), "refresh_token".to_string()),
                ("refresh_token".to_string(), "rt-old".to_string()),
                (
                    "redirect_uri".to_string(),
                    "https://sapa-tv.ru/login-callback/twitch".to_string()
                ),
            ]
        );
        assert_eq!(token.access_token, "at2");
    }

    #[tokio::test]
    async fn revoke_sends_token_and_hint() {
        let transport = FakeTransport::new(Ok("{}".to_string()));
        client()
            .revoke(&transport, "tok-1", TokenHint::Refresh)
            .await
            .unwrap();

        let record = transport.last();
        assert_eq!(record.url, REVOKE_URL);
        assert_eq!(
            record.form,
            vec![
                ("token".to_string(), "tok-1".to_string()),
                ("token_type_hint".to_string(), "refresh_token".to_string()),
            ]
        );
    }

    #[tokio::test]
    async fn transport_error_propagates() {
        let transport = FakeTransport::new(Err(Error::Http("status 400: bad".to_string())));
        let res = client()
            .exchange_code(&transport, "c", "https://x.example/cb")
            .await;
        match res {
            Err(Error::Http(m)) => assert_eq!(m, "status 400: bad"),
            other => panic!("expected http error, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn malformed_token_body_is_protocol_error() {
        let transport = FakeTransport::new(Ok("not json".to_string()));
        let res = client()
            .exchange_code(&transport, "c", "https://x.example/cb")
            .await;
        assert!(matches!(res, Err(Error::Protocol(_))));
    }
}
