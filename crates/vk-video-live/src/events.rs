use serde_json::Value;

use crate::error::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ChatMessageEvent {
    pub id: u64,
    pub author_id: u64,
    pub author_nick: String,
    pub created_at: i64,
    pub text: String,
    pub is_deleted: bool,
    pub is_private: bool,
}

impl ChatMessageEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: u64,
        author_id: u64,
        author_nick: impl Into<String>,
        created_at: i64,
        text: impl Into<String>,
        is_deleted: bool,
        is_private: bool,
    ) -> Self {
        Self {
            id,
            author_id,
            author_nick: author_nick.into(),
            created_at,
            text: text.into(),
            is_deleted,
            is_private,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct RewardDemandEvent {
    pub id: u64,
    pub user_id: u64,
    pub user_nick: String,
    pub reward_id: String,
    pub status: String,
    pub created_at: i64,
}

impl RewardDemandEvent {
    pub fn new(
        id: u64,
        user_id: u64,
        user_nick: impl Into<String>,
        reward_id: impl Into<String>,
        status: impl Into<String>,
        created_at: i64,
    ) -> Self {
        Self {
            id,
            user_id,
            user_nick: user_nick.into(),
            reward_id: reward_id.into(),
            status: status.into(),
            created_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PushEvent {
    ChatMessage(ChatMessageEvent),
    RewardDemand(RewardDemandEvent),
}

pub fn parse_push(frame: &str) -> Result<Option<PushEvent>> {
    let value: Value =
        serde_json::from_str(frame).map_err(|e| Error::Protocol(format!("ws frame: {e}")))?;
    let Some(data) = value.pointer("/push/pub/data") else {
        return Ok(None);
    };
    match data.get("type").and_then(Value::as_str) {
        Some("channel_chat_message_send") => parse_chat_message(data),
        Some("channel_points_reward_demand_create") => parse_reward_demand(data),
        _ => Ok(None),
    }
}

fn parse_chat_message(data: &Value) -> Result<Option<PushEvent>> {
    let Some(msg) = data.pointer("/data/chat_message") else {
        return Ok(None);
    };
    let text = msg
        .get("parts")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|p| p.pointer("/text/content").and_then(Value::as_str))
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default();
    Ok(Some(PushEvent::ChatMessage(ChatMessageEvent {
        id: required_u64(msg, "id")?,
        author_id: msg
            .pointer("/author/id")
            .and_then(as_id)
            .ok_or_else(|| Error::Protocol("author.id missing".to_string()))?,
        author_nick: msg
            .pointer("/author/nick")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        created_at: required_i64(msg, "created_at")?,
        text,
        is_deleted: false,
        is_private: msg
            .get("is_private")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })))
}

fn parse_reward_demand(data: &Value) -> Result<Option<PushEvent>> {
    let Some(demand) = data.pointer("/data/demand") else {
        return Ok(None);
    };
    Ok(Some(PushEvent::RewardDemand(RewardDemandEvent {
        id: required_u64(demand, "id")?,
        user_id: demand
            .pointer("/user/id")
            .and_then(as_id)
            .ok_or_else(|| Error::Protocol("user.id missing".to_string()))?,
        user_nick: demand
            .pointer("/user/nick")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        reward_id: demand
            .pointer("/reward/id")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Protocol("reward.id missing".to_string()))?
            .to_string(),
        status: demand
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        created_at: required_i64(demand, "created_at")?,
    })))
}

fn required_u64(value: &Value, key: &str) -> Result<u64> {
    value
        .get(key)
        .and_then(as_id)
        .ok_or_else(|| Error::Protocol(format!("{key} missing")))
}

fn as_id(value: &Value) -> Option<u64> {
    match value {
        Value::Number(n) => n.as_u64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

fn required_i64(value: &Value, key: &str) -> Result<i64> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| Error::Protocol(format!("{key} missing")))
}

#[cfg(test)]
#[path = "events.test.rs"]
mod tests;
