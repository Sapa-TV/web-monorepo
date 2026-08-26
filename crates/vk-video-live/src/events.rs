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

pub fn parse_push(frame: &str) -> Result<Option<ChatMessageEvent>> {
    let value: Value =
        serde_json::from_str(frame).map_err(|e| Error::Protocol(format!("ws frame: {e}")))?;
    let Some(channel) = value.pointer("/push/channel").and_then(Value::as_str) else {
        return Ok(None);
    };
    if !channel.starts_with("channel-chat:") {
        return Ok(None);
    }
    let Some(data) = value.pointer("/push/pub/data") else {
        return Ok(None);
    };
    match data.get("type").and_then(Value::as_str) {
        Some("message") => parse_legacy(data),
        Some("message_v8") => parse_v8(data),
        _ => Ok(None),
    }
}

fn parse_legacy(data: &Value) -> Result<Option<ChatMessageEvent>> {
    let Some(msg) = data.get("data") else {
        return Ok(None);
    };
    let author = msg.get("author").or_else(|| msg.get("user"));
    Ok(Some(ChatMessageEvent {
        id: required_u64(msg, "id")?,
        author_id: author
            .and_then(|a| a.get("id"))
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::Protocol("author.id missing".to_string()))?,
        author_nick: author
            .and_then(|a| a.get("nick").or_else(|| a.get("displayName")))
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        created_at: required_i64(msg, "createdAt")?,
        text: blocks_text(msg.get("data")),
        is_deleted: flag(msg, "isDeleted"),
        is_private: flag(msg, "isPrivate"),
    }))
}

fn parse_v8(data: &Value) -> Result<Option<ChatMessageEvent>> {
    let Some(msg) = data.pointer("/data/chatMessageSend/message") else {
        return Ok(None);
    };
    let text = msg
        .get("text")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| blocks_text(msg.get("textData")));
    Ok(Some(ChatMessageEvent {
        id: required_u64(msg, "id")?,
        author_id: msg
            .pointer("/author/id")
            .and_then(Value::as_u64)
            .ok_or_else(|| Error::Protocol("author.id missing".to_string()))?,
        author_nick: msg
            .pointer("/author/nick")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string(),
        created_at: required_i64(msg, "createdAt")?,
        text,
        is_deleted: flag(msg, "isDeleted"),
        is_private: flag(msg, "isPrivate"),
    }))
}

fn blocks_text(blocks: Option<&Value>) -> String {
    let Some(blocks) = blocks.and_then(Value::as_array) else {
        return String::new();
    };
    blocks
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("content").and_then(Value::as_str))
        .map(decode_content)
        .filter(|text| !text.is_empty())
        .collect()
}

fn decode_content(content: &str) -> String {
    let Ok(parsed) = serde_json::from_str::<Value>(content) else {
        return content.to_string();
    };
    match parsed.as_array().and_then(|items| items.first()) {
        Some(Value::String(text)) => text.clone(),
        _ => content.to_string(),
    }
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

fn flag(value: &Value, key: &str) -> bool {
    value
        .pointer(&format!("/flags/{key}"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    const LEGACY_FRAME: &str = r#"{"push":{"channel":"channel-chat:12414691","pub":{"data":{"type":"message","data":{"parent":null,"id":651722866,"createdAt":1787682950,"styles":[],"flags":{"isParentDeleted":false,"isFirstMessage":false},"author":{"id":29605551,"nick":"Th0r_N13","name":"Th0r_N13","displayName":"Th0r_N13","nickColor":13,"isChannelModerator":true,"roles":[]},"isDeleted":false,"isPrivate":false,"data":[{"type":"text","content":"[\"ку\",\"unstyled\",[]]","modificator":"","donation":false},{"type":"text","content":"","modificator":"BLOCK_END","donation":false}],"threadId":null,"streamSlot":null,"user":{"id":29605551,"nick":"Th0r_N13"}}}}}}"#;

    const V8_FRAME: &str = r#"{"push":{"channel":"channel-chat:12414691","pub":{"data":{"type":"message_v8","data":{"chatMessageSend":{"message":{"id":"651722866","author":{"id":29605551,"nick":"Th0r_N13","nickColor":13,"roles":[]},"createdAt":1787682950,"textData":[{"text":{"type":"text","content":"[\"ку\",\"unstyled\",[]]","modificator":"","donation":false}},{"text":{"type":"text","content":"","modificator":"BLOCK_END","donation":false}}],"text":"ку","styles":[],"flags":{"isDeleted":false,"isParentDeleted":false,"isFirstMessage":false,"isPrivate":false}}}}}}}}"#;

    fn expected() -> ChatMessageEvent {
        ChatMessageEvent {
            id: 651722866,
            author_id: 29605551,
            author_nick: "Th0r_N13".to_string(),
            created_at: 1_787_682_950,
            text: "ку".to_string(),
            is_deleted: false,
            is_private: false,
        }
    }

    #[test]
    fn legacy_message_frame_parses() {
        let event = parse_push(LEGACY_FRAME).unwrap().unwrap();
        assert_eq!(event, expected());
    }

    #[test]
    fn v8_message_frame_parses() {
        let event = parse_push(V8_FRAME).unwrap().unwrap();
        assert_eq!(event, expected());
    }

    #[test]
    fn non_chat_channel_is_ignored() {
        let frame = r#"{"push":{"channel":"channel-info:12414691","pub":{"data":{"type":"stream_status","data":{}}}}}"#;
        assert_eq!(parse_push(frame).unwrap(), None);
    }

    #[test]
    fn non_push_frame_is_ignored() {
        assert_eq!(parse_push(r#"{"id":2,"subscribe":{}}"#).unwrap(), None);
    }

    #[test]
    fn unknown_message_type_is_ignored() {
        let frame = r#"{"push":{"channel":"channel-chat:1","pub":{"data":{"type":"future_thing","data":{}}}}}"#;
        assert_eq!(parse_push(frame).unwrap(), None);
    }

    #[test]
    fn deleted_flag_is_parsed() {
        let frame = r#"{"push":{"channel":"channel-chat:1","pub":{"data":{"type":"message","data":{"id":1,"createdAt":10,"author":{"id":2,"nick":"n"},"flags":{"isDeleted":true,"isPrivate":true},"data":[]}}}}}"#;
        let event = parse_push(frame).unwrap().unwrap();
        assert!(event.is_deleted);
        assert!(event.is_private);
        assert_eq!(event.text, "");
    }

    #[test]
    fn plain_content_stays_untouched() {
        let frame = r#"{"push":{"channel":"channel-chat:1","pub":{"data":{"type":"message","data":{"id":1,"createdAt":10,"author":{"id":2,"nick":"n"},"data":[{"type":"text","content":"plain"}]}}}}}"#;
        let event = parse_push(frame).unwrap().unwrap();
        assert_eq!(event.text, "plain");
    }

    #[test]
    fn malformed_frame_is_protocol_error() {
        assert!(matches!(parse_push("not json"), Err(Error::Protocol(_))));
    }

    #[test]
    fn missing_id_is_protocol_error() {
        let frame = r#"{"push":{"channel":"channel-chat:1","pub":{"data":{"type":"message","data":{"createdAt":10,"author":{"id":2,"nick":"n"}}}}}}"#;
        assert!(matches!(parse_push(frame), Err(Error::Protocol(_))));
    }
}
