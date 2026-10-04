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
    async fn post_form(&self, url: &str, basic_auth: &str, encoded_body: &str) -> Result<String> {
        self.records.lock().unwrap().push(Record {
            url: url.to_string(),
            auth: basic_auth.to_string(),
            form: vec![("body".to_string(), encoded_body.to_string())],
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
        vec![(
            "body".to_string(),
            "grant_type=authorization_code\
&code=the-code\
&redirect_uri=https%3A%2F%2Fsapa-tv.ru%2Flogin-callback%2Ftwitch"
                .to_string()
        ),]
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
        vec![(
            "body".to_string(),
            "grant_type=refresh_token\
&refresh_token=rt-old\
&redirect_uri=https%3A%2F%2Fsapa-tv.ru%2Flogin-callback%2Ftwitch"
                .to_string()
        ),]
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
        vec![(
            "body".to_string(),
            "token=tok-1&token_type_hint=refresh_token".to_string()
        ),]
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
