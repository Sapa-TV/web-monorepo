use super::*;

const CHAT_FRAME: &str = r#"{"push":{"channel":"api-channel-chat:4242","pub":{"data":{"type":"channel_chat_message_send","data":{"chat_message":{"id":9001,"created_at":1787682950,"is_private":false,"author":{"id":555,"nick":"tester"},"parts":[{"text":{"content":"ку"}}]}}}}}}"#;

const DEMAND_FRAME: &str = r#"{"push":{"channel":"api-channel-points:4242#555","pub":{"data":{"type":"channel_points_reward_demand_create","data":{"demand":{"id":9179,"user":{"id":555,"nick":"tester"},"message_parts":[],"status":"pending","created_at":1787682950,"reward":{"id":"bc211809-1e52-4180-8dbc-e106649ef78a"}}}}}}}"#;

#[test]
fn chat_message_frame_parses() {
    let event = parse_push(CHAT_FRAME).unwrap().unwrap();
    assert_eq!(
        event,
        PushEvent::ChatMessage(ChatMessageEvent {
            id: 9001,
            author_id: 555,
            author_nick: "tester".to_string(),
            created_at: 1_787_682_950,
            text: "ку".to_string(),
            is_deleted: false,
            is_private: false,
        })
    );
}

#[test]
fn private_chat_message_flag_is_parsed() {
    let frame = r#"{"push":{"channel":"api-channel-chat:4242#555","pub":{"data":{"type":"channel_chat_message_send","data":{"chat_message":{"id":1,"created_at":10,"is_private":true,"author":{"id":2,"nick":"n"},"parts":[]}}}}}}"#;
    let Some(PushEvent::ChatMessage(event)) = parse_push(frame).unwrap() else {
        panic!("expected chat message");
    };
    assert!(event.is_private);
}

#[test]
fn reward_demand_frame_parses() {
    let event = parse_push(DEMAND_FRAME).unwrap().unwrap();
    assert_eq!(
        event,
        PushEvent::RewardDemand(RewardDemandEvent {
            id: 9179,
            user_id: 555,
            user_nick: "tester".to_string(),
            reward_id: "bc211809-1e52-4180-8dbc-e106649ef78a".to_string(),
            status: "pending".to_string(),
            created_at: 1_787_682_950,
        })
    );
}

#[test]
fn unknown_push_type_is_ignored() {
    let frame = r#"{"push":{"channel":"api-channel-info:1","pub":{"data":{"type":"stream_status","data":{}}}}}"#;
    assert_eq!(parse_push(frame).unwrap(), None);
}

#[test]
fn non_push_frame_is_ignored() {
    assert_eq!(parse_push(r#"{"id":2,"subscribe":{}}"#).unwrap(), None);
}

#[test]
fn malformed_frame_is_protocol_error() {
    assert!(matches!(parse_push("not json"), Err(Error::Protocol(_))));
}

#[test]
fn missing_id_is_protocol_error() {
    let frame = r#"{"push":{"channel":"api-channel-chat:1","pub":{"data":{"type":"channel_chat_message_send","data":{"chat_message":{"created_at":10,"author":{"id":2,"nick":"n"},"parts":[]}}}}}}"#;
    assert!(matches!(parse_push(frame), Err(Error::Protocol(_))));
}
