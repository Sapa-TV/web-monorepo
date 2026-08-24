use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use better_tokio_select::tokio_select;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tokio::sync::broadcast;

use crate::presence::{PresenceSnapshot, WsClientRole};
use crate::queue::entry::QueueEntryId;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
enum ClientMessage {
    #[serde(rename = "auth")]
    Auth { token: String, role: WsClientRole },
    #[serde(rename = "complete")]
    Complete { entry_id: QueueEntryId },
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
#[non_exhaustive]
enum ServerMessage {
    #[serde(rename = "auth_ok")]
    AuthOk,
    #[serde(rename = "auth_err")]
    AuthErr,
    #[serde(rename = "complete_ok")]
    CompleteOk { entry_id: QueueEntryId },
    #[serde(rename = "complete_err")]
    CompleteErr {
        entry_id: QueueEntryId,
        error: String,
    },
    #[serde(rename = "presence")]
    Presence { dock: bool, widget_count: usize },
}

impl ServerMessage {
    fn presence(snapshot: PresenceSnapshot) -> Self {
        Self::Presence {
            dock: snapshot.dock > 0,
            widget_count: snapshot.widget,
        }
    }
}

fn validate_token(state: &AppState, token: &str) -> bool {
    token
        .as_bytes()
        .ct_eq(state.config.widget_access_key().as_bytes())
        .into()
}

async fn handle_message(state: &AppState, msg: ClientMessage) -> ServerMessage {
    match msg {
        ClientMessage::Auth { token, .. } => {
            if validate_token(state, &token) {
                ServerMessage::AuthOk
            } else {
                ServerMessage::AuthErr
            }
        }
        ClientMessage::Complete { entry_id } => {
            match state.queue_service.complete(entry_id).await {
                Ok(()) => ServerMessage::CompleteOk { entry_id },
                Err(e) => ServerMessage::CompleteErr {
                    entry_id,
                    error: e.to_string(),
                },
            }
        }
    }
}

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let Some(Ok(Message::Text(text))) = socket.recv().await else {
        return;
    };
    let Ok(msg) = serde_json::from_str::<ClientMessage>(&text) else {
        return;
    };
    let ClientMessage::Auth { role, .. } = &msg else {
        return;
    };
    let role = *role;
    let reply = handle_message(&state, msg).await;
    let Ok(json) = serde_json::to_string(&reply) else {
        return;
    };
    if socket.send(Message::Text(json.into())).await.is_err() {
        return;
    }
    if !matches!(reply, ServerMessage::AuthOk) {
        return;
    }

    let _presence_guard = state.presence.add(role);
    let mut presence_rx = state.presence.subscribe();
    let initial = ServerMessage::presence(state.presence.snapshot());
    if !send_json(&mut socket, &initial).await {
        return;
    }

    let mut rx = state.event_publisher.subscribe();

    tracing::info!(role = role.as_ref(), "ws client connected");

    loop {
        tokio_select!(match .. {
            .. if let changed = presence_rx.changed() => match changed {
                Ok(()) => {
                    let snapshot = *presence_rx.borrow_and_update();
                    if !send_json(&mut socket, &ServerMessage::presence(snapshot)).await {
                        break;
                    }
                }
                Err(_) => break,
            },
            .. if let result = rx.recv() => match result {
                Ok(event) => {
                    let Ok(json) = serde_json::to_string(&*event) else {
                        break;
                    };
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("ws client lagged, skipped {n} events");
                }
                Err(broadcast::error::RecvError::Closed) => break,
            },
            .. if let msg = socket.recv() => match msg {
                Some(Ok(Message::Text(text))) => {
                    let Ok(msg) = serde_json::from_str::<ClientMessage>(&text) else {
                        continue;
                    };
                    let reply = handle_message(&state, msg).await;
                    let Ok(json) = serde_json::to_string(&reply) else {
                        continue;
                    };
                    if socket.send(Message::Text(json.into())).await.is_err() {
                        break;
                    }
                }
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            },
        })
    }

    tracing::debug!(role = role.as_ref(), "ws client disconnected");
}

async fn send_json(socket: &mut WebSocket, message: &ServerMessage) -> bool {
    match serde_json::to_string(message) {
        Ok(json) => socket.send(Message::Text(json.into())).await.is_ok(),
        Err(_) => false,
    }
}

pub fn public_router() -> axum::Router<AppState> {
    axum::Router::new().route("/ws", get(ws_handler))
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::presence::{PresenceSnapshot, WsClientRole};
    use crate::queue::entry::{QueueEntryId, QueueStatus};
    use crate::roulette::rarity::{Rarity, RarityId};
    use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
    use crate::state::AppState;
    use crate::test_fixtures::test_state;
    use crate::widget_api::ws::{ClientMessage, ServerMessage, handle_message};

    async fn setup_spinning(state: &AppState) -> QueueEntryId {
        state
            .rarity_service
            .save(Rarity::new(
                RarityId::new(1),
                "common",
                "Common",
                "c.png",
                "#fff",
            ))
            .await
            .unwrap();
        state
            .slot_service
            .add_slot(RouletteSlot::new(
                RouletteSlotId::new(0),
                "test_slot",
                RarityId::new(1),
                100,
                "test",
            ))
            .await
            .unwrap();
        let user_id = state.user_service.create("user1").await.unwrap().id;
        state.queue_service.enqueue(user_id, "user1").await.unwrap();
        let (entry, _slot) = state.queue_service.dequeue_next().await.unwrap();
        entry.id
    }

    #[tokio::test]
    async fn auth_handshake_validates_token() {
        let state = test_state().await;

        let ok = handle_message(
            &state,
            ClientMessage::Auth {
                token: "test-key".to_string(),
                role: WsClientRole::Dock,
            },
        )
        .await;
        assert!(matches!(ok, ServerMessage::AuthOk));

        let bad = handle_message(
            &state,
            ClientMessage::Auth {
                token: "wrong-key".to_string(),
                role: WsClientRole::Widget,
            },
        )
        .await;
        assert!(matches!(bad, ServerMessage::AuthErr));
    }

    #[test]
    fn auth_without_role_is_rejected_by_deserialization() {
        let parsed = serde_json::from_str::<ClientMessage>(r#"{"type":"auth","token":"k"}"#);
        assert!(parsed.is_err(), "auth requires a role field");

        let parsed =
            serde_json::from_str::<ClientMessage>(r#"{"type":"auth","token":"k","role":"widget"}"#);
        assert!(parsed.is_ok());
    }

    #[tokio::test]
    async fn presence_snapshot_serializes_with_type_tag() {
        let msg = ServerMessage::presence(PresenceSnapshot { dock: 2, widget: 3 });
        let json: Value = serde_json::to_value(&msg).unwrap();
        assert_eq!(json["type"], "presence");
        assert_eq!(json["dock"], true);
        assert_eq!(json["widget_count"], 3);

        let empty = ServerMessage::presence(PresenceSnapshot::default());
        let json: Value = serde_json::to_value(&empty).unwrap();
        assert_eq!(json["dock"], false);
        assert_eq!(json["widget_count"], 0);
    }

    #[tokio::test]
    async fn complete_via_message_marks_entry_completed_once() {
        let state = test_state().await;

        let entry_id = setup_spinning(&state).await;
        let reply = handle_message(&state, ClientMessage::Complete { entry_id }).await;
        assert!(matches!(reply, ServerMessage::CompleteOk { entry_id: id } if id == entry_id));
        let entry = state
            .queue_service
            .get_by_id(entry_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.status, QueueStatus::Completed);

        let reply = handle_message(&state, ClientMessage::Complete { entry_id }).await;
        assert!(matches!(
            reply,
            ServerMessage::CompleteErr { entry_id: id, error }
            if id == entry_id && !error.is_empty()
        ));
    }

    #[tokio::test]
    async fn complete_ok_and_err_serialize_with_type_tag() {
        let ok = ServerMessage::CompleteOk {
            entry_id: QueueEntryId::new(7),
        };
        let json: Value = serde_json::to_value(&ok).unwrap();
        assert_eq!(json["type"], "complete_ok");
        assert_eq!(json["entry_id"], 7);

        let err = ServerMessage::CompleteErr {
            entry_id: QueueEntryId::new(7),
            error: "nope".to_string(),
        };
        let json: Value = serde_json::to_value(&err).unwrap();
        assert_eq!(json["type"], "complete_err");
        assert_eq!(json["error"], "nope");
    }
}
