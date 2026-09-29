use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use crate::events::ChatMessageEvent;
use crate::transport::Transport;
use crate::url::append_query;

pub const API_BASE: &str = "https://api.live.vkvideo.ru";

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ChannelResponse {
    pub data: ChannelData,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ChannelData {
    pub channel: ChannelInfo,
    pub owner: Option<ChannelOwner>,
    pub stream: Option<StreamInfo>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ChannelOwner {
    pub id: u64,
    pub nick: String,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct ChannelInfo {
    pub id: u64,
    pub url: String,
    pub nick: String,
    pub web_socket_channels: WsChannels,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct WsChannels {
    pub chat: Option<String>,
    pub info: Option<String>,
    pub channel_points: Option<String>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct StreamInfo {
    pub id: String,
    pub status: Option<String>,
    pub started_at: Option<i64>,
    pub counters: Option<StreamCounters>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct StreamCounters {
    pub viewers: Option<u64>,
    #[serde(skip)]
    _sealed: (),
}

pub async fn channel<T: Transport>(
    transport: &T,
    bearer: &str,
    channel_url: &str,
) -> Result<ChannelResponse> {
    let url = append_query(
        &format!("{API_BASE}/v1/channel"),
        &[("channel_url", channel_url)],
    );
    let body = transport.get(&url, bearer).await?;
    parse(&body)
}

pub async fn ws_token<T: Transport>(transport: &T, bearer: &str) -> Result<String> {
    let body = transport
        .get(&format!("{API_BASE}/v1/websocket/token"), bearer)
        .await?;
    let value: serde_json::Value = parse(&body)?;
    value
        .pointer("/data/token")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Error::Protocol("websocket token missing".to_string()))
}

pub async fn send_chat_message<T: Transport>(
    transport: &T,
    bearer: &str,
    channel_url: &str,
    stream_id: &str,
    text: &str,
) -> Result<()> {
    let url = append_query(
        &format!("{API_BASE}/v1/chat/message/send"),
        &[("channel_url", channel_url), ("stream_id", stream_id)],
    );
    let body = serde_json::json!({ "parts": [{ "text": { "content": text } }] }).to_string();
    transport.post_json(&url, bearer, &body).await?;
    Ok(())
}

pub async fn chat_messages<T: Transport>(
    transport: &T,
    bearer: &str,
    channel_url: &str,
    limit: u32,
) -> Result<Vec<ChatMessageEvent>> {
    let limit = limit.clamp(1, 200);
    let url = append_query(
        &format!("{API_BASE}/v1/chat/messages"),
        &[("channel_url", channel_url), ("limit", &limit.to_string())],
    );
    let body = transport.get(&url, bearer).await?;
    let value: serde_json::Value = parse(&body)?;
    let messages = value
        .pointer("/data/chat_messages")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| Error::Protocol("chat_messages missing".to_string()))?;

    messages
        .iter()
        .map(|m| {
            Ok(ChatMessageEvent {
                id: field_u64(m, "id")?,
                author_id: m
                    .pointer("/author/id")
                    .and_then(serde_json::Value::as_u64)
                    .ok_or_else(|| Error::Protocol("author.id missing".to_string()))?,
                author_nick: m
                    .pointer("/author/nick")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                created_at: field_i64(m, "created_at")?,
                text: m
                    .get("parts")
                    .and_then(serde_json::Value::as_array)
                    .map(|parts| {
                        parts
                            .iter()
                            .filter_map(|p| {
                                p.pointer("/text/content")
                                    .and_then(serde_json::Value::as_str)
                            })
                            .collect::<Vec<_>>()
                            .join("")
                    })
                    .unwrap_or_default(),
                is_deleted: false,
                is_private: m
                    .get("is_private")
                    .and_then(serde_json::Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

fn field_u64(value: &serde_json::Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(serde_json::Value::as_u64)
        .ok_or_else(|| Error::Protocol(format!("{key} missing")))
}

fn field_i64(value: &serde_json::Value, key: &str) -> Result<i64> {
    value
        .get(key)
        .and_then(serde_json::Value::as_i64)
        .ok_or_else(|| Error::Protocol(format!("{key} missing")))
}

fn parse<T: DeserializeOwned>(body: &str) -> Result<T> {
    serde_json::from_str(body).map_err(|e| Error::Protocol(format!("api response: {e}")))
}

#[cfg(test)]
mod tests {
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
}
