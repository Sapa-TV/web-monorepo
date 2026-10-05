use serde::Deserialize;
use serde::de::DeserializeOwned;

use crate::error::{Error, Result};
use crate::events::ChatMessageEvent;
use crate::transport::Transport;
use crate::url::append_query;

pub const API_BASE: &str = "https://apidev.live.vkvideo.ru";

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
    pub private_chat: Option<String>,
    pub limited_chat: Option<String>,
    pub limited_private_chat: Option<String>,
    pub info: Option<String>,
    pub private_info: Option<String>,
    pub channel_points: Option<String>,
    pub private_channel_points: Option<String>,
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

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct CurrentUserResponse {
    pub data: CurrentUserData,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct CurrentUserData {
    pub channel: Option<CurrentUserChannel>,
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Clone, Deserialize)]
#[non_exhaustive]
pub struct CurrentUserChannel {
    pub url: String,
    #[serde(skip)]
    _sealed: (),
}

pub async fn current_user<T: Transport>(
    transport: &T,
    bearer: &str,
) -> Result<CurrentUserResponse> {
    let body = transport
        .get(&format!("{API_BASE}/v1/current_user"), bearer)
        .await?;
    parse(&body)
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
#[path = "api.test.rs"]
mod tests;
