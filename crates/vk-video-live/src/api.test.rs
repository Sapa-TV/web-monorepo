use std::sync::{Arc, Mutex};

use super::*;
use crate::error::Error;

#[derive(Debug, Clone)]
struct Call {
    url: String,
    bearer: String,
    body: Option<String>,
}

#[derive(Debug, Clone)]
struct FakeApi {
    response: Result<String>,
    calls: Arc<Mutex<Vec<Call>>>,
}

impl FakeApi {
    fn new(response: Result<String>) -> Self {
        Self {
            response,
            calls: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn last(&self) -> Call {
        self.calls.lock().unwrap().last().unwrap().clone()
    }
}

impl Transport for FakeApi {
    async fn post_form(
        &self,
        _url: &str,
        _basic_auth: &str,
        _encoded_body: &str,
    ) -> Result<String> {
        Err(Error::Protocol("unexpected post_form".to_string()))
    }

    async fn get(&self, url: &str, bearer: &str) -> Result<String> {
        self.calls.lock().unwrap().push(Call {
            url: url.to_string(),
            bearer: bearer.to_string(),
            body: None,
        });
        self.response.clone()
    }

    async fn post_json(&self, url: &str, bearer: &str, body: &str) -> Result<String> {
        self.calls.lock().unwrap().push(Call {
            url: url.to_string(),
            bearer: bearer.to_string(),
            body: Some(body.to_string()),
        });
        self.response.clone()
    }
}

const BEARER: &str = "Bearer tok";
const TEXT_HI: &str = "\u{43a}\u{443}";

#[tokio::test]
async fn channel_parses_ids_and_ws_channels() {
    let transport = FakeApi::new(Ok(
        r#"{"data":{"channel":{"id":4242,"url":"test_channel","nick":"TestChannel",
            "web_socket_channels":{"chat":"channel-chat:4242","info":"channel-info:4242",
            "channel_points":"channel-points:4242"}},
            "stream":{"id":"s1","status":"online","started_at":1787501283,
            "counters":{"viewers":37}}}}"#
            .replace('\n', "")
            .to_string(),
    ));
    let res = channel(&transport, BEARER, "test_channel").await.unwrap();

    let call = transport.last();
    assert_eq!(
        call.url,
        format!("{API_BASE}/v1/channel?channel_url=test_channel")
    );
    assert_eq!(call.bearer, BEARER);
    assert_eq!(res.data.channel.id, 4242);
    assert_eq!(
        res.data.channel.web_socket_channels.chat.as_deref(),
        Some("channel-chat:4242")
    );
    let stream = res.data.stream.unwrap();
    assert_eq!(stream.id, "s1");
    assert_eq!(stream.counters.unwrap().viewers, Some(37));
}

#[tokio::test]
async fn channel_stream_can_be_absent() {
    let transport = FakeApi::new(Ok(r#"{"data":{"channel":{"id":1,"url":"u","nick":"n",
            "web_socket_channels":{}},"stream":null}}"#
        .to_string()));
    let res = channel(&transport, BEARER, "u").await.unwrap();
    assert!(res.data.stream.is_none());
    assert!(res.data.channel.web_socket_channels.chat.is_none());
}

#[tokio::test]
async fn ws_token_extracts_data_token() {
    let transport = FakeApi::new(Ok(r#"{"data":{"token":"jwt"}}"#.to_string()));
    let token = ws_token(&transport, BEARER).await.unwrap();
    assert_eq!(
        transport.last().url,
        format!("{API_BASE}/v1/websocket/token")
    );
    assert_eq!(token, "jwt");
}

#[tokio::test]
async fn send_chat_message_posts_parts_payload() {
    let transport = FakeApi::new(Ok("{}".to_string()));
    send_chat_message(&transport, BEARER, "test_channel", "s1", TEXT_HI)
        .await
        .unwrap();

    let call = transport.last();
    assert_eq!(
        call.url,
        format!("{API_BASE}/v1/chat/message/send?channel_url=test_channel&stream_id=s1")
    );
    let expected = format!(r#"{{"parts":[{{"text":{{"content":"{TEXT_HI}"}}}}]}}"#);
    assert_eq!(call.body.as_deref(), Some(expected.as_str()));
}

#[tokio::test]
async fn chat_messages_maps_history() {
    let transport = FakeApi::new(Ok(r#"{"data":{"chat_messages":[
            {"id":11,"created_at":1787682744,"is_private":false,
             "author":{"id":555,"nick":"tester"},
             "parts":[{"text":{"content":"hi"}},{"text":{"content":"!"}}]},
            {"id":12,"created_at":1787682745,"is_private":true,
             "author":{"id":7,"nick":"anon"},"parts":[]}]}}"#
        .replace('\n', "")
        .to_string()));
    let messages = chat_messages(&transport, BEARER, "test_channel", 50)
        .await
        .unwrap();

    let call = transport.last();
    assert!(
        call.url
            .ends_with("/v1/chat/messages?channel_url=test_channel&limit=50")
    );
    assert_eq!(messages.len(), 2);
    assert_eq!(messages[0].id, 11);
    assert_eq!(messages[0].text, "hi!");
    assert_eq!(messages[0].author_nick, "tester");
    assert!(messages[1].is_private);
    assert_eq!(messages[1].text, "");
}

#[tokio::test]
async fn chat_messages_limit_clamped() {
    let transport = FakeApi::new(Ok(r#"{"data":{"chat_messages":[]}}"#.to_string()));
    chat_messages(&transport, BEARER, "u", 500).await.unwrap();
    assert!(transport.last().url.ends_with("limit=200"));
}

#[tokio::test]
async fn malformed_response_is_protocol_error() {
    let transport = FakeApi::new(Ok("oops".to_string()));
    assert!(matches!(
        channel(&transport, BEARER, "u").await,
        Err(Error::Protocol(_))
    ));
}
