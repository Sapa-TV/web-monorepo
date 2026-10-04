use super::*;

const LEGACY_FRAME: &str = r#"{"push":{"channel":"channel-chat:4242","pub":{"data":{"type":"message","data":{"parent":null,"id":9001,"createdAt":1787682950,"styles":[],"flags":{"isParentDeleted":false,"isFirstMessage":false},"author":{"id":555,"nick":"tester","name":"tester","displayName":"tester","nickColor":13,"isChannelModerator":true,"roles":[]},"isDeleted":false,"isPrivate":false,"data":[{"type":"text","content":"[\"ку\",\"unstyled\",[]]","modificator":"","donation":false},{"type":"text","content":"","modificator":"BLOCK_END","donation":false}],"threadId":null,"streamSlot":null,"user":{"id":555,"nick":"tester"}}}}}}"#;

const V8_FRAME: &str = r#"{"push":{"channel":"channel-chat:4242","pub":{"data":{"type":"message_v8","data":{"chatMessageSend":{"message":{"id":"9001","author":{"id":555,"nick":"tester","nickColor":13,"roles":[]},"createdAt":1787682950,"textData":[{"text":{"type":"text","content":"[\"ку\",\"unstyled\",[]]","modificator":"","donation":false}},{"text":{"type":"text","content":"","modificator":"BLOCK_END","donation":false}}],"text":"ку","styles":[],"flags":{"isDeleted":false,"isParentDeleted":false,"isFirstMessage":false,"isPrivate":false}}}}}}}}"#;

fn expected() -> ChatMessageEvent {
    ChatMessageEvent {
        id: 9001,
        author_id: 555,
        author_nick: "tester".to_string(),
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
    let frame = r#"{"push":{"channel":"channel-info:4242","pub":{"data":{"type":"stream_status","data":{}}}}}"#;
    assert_eq!(parse_push(frame).unwrap(), None);
}

#[test]
fn non_push_frame_is_ignored() {
    assert_eq!(parse_push(r#"{"id":2,"subscribe":{}}"#).unwrap(), None);
}

#[test]
fn unknown_message_type_is_ignored() {
    let frame =
        r#"{"push":{"channel":"channel-chat:1","pub":{"data":{"type":"future_thing","data":{}}}}}"#;
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
