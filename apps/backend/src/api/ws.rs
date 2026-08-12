use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::get;
use better_tokio_select::tokio_select;
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
use tokio::sync::broadcast;

use crate::queue::entry::QueueEntryId;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
#[non_exhaustive]
enum ClientMessage {
    #[serde(rename = "auth")]
    Auth { token: String },
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
}

async fn handle_message(state: &AppState, msg: ClientMessage) -> ServerMessage {
    match msg {
        ClientMessage::Auth { token } => {
            let authorized: bool = token
                .as_bytes()
                .ct_eq(state.config.access_key().as_bytes())
                .into();
            if authorized {
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

    let mut rx = state.event_publisher.subscribe();

    tracing::info!("ws client connected");

    loop {
        tokio_select!(match .. {
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

    tracing::debug!("ws client disconnected");
}

pub fn public_router() -> axum::Router<AppState> {
    axum::Router::new().route("/ws", get(ws_handler))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::http::Request;
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::api::router;
    use crate::api::ws::{ClientMessage, ServerMessage, handle_message};
    use crate::queue::entry::{QueueEntryId, QueueStatus};
    use crate::roulette::rarity::{Rarity, RarityId};
    use crate::roulette::slot_service::{RouletteSlot, RouletteSlotId};
    use crate::state::AppState;
    use crate::test_fixtures::test_state;

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
            },
        )
        .await;
        assert!(matches!(ok, ServerMessage::AuthOk));

        let bad = handle_message(
            &state,
            ClientMessage::Auth {
                token: "wrong-key".to_string(),
            },
        )
        .await;
        assert!(matches!(bad, ServerMessage::AuthErr));
    }

    #[tokio::test]
    async fn ws_and_rest_complete_are_equivalent() {
        let state = test_state().await;
        let app = router(state.clone());

        let ws_entry_id = setup_spinning(&state).await;
        let reply = handle_message(
            &state,
            ClientMessage::Complete {
                entry_id: ws_entry_id,
            },
        )
        .await;
        assert!(matches!(reply, ServerMessage::CompleteOk { entry_id } if entry_id == ws_entry_id));
        let entry = state
            .queue_service
            .get_by_id(ws_entry_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.status, QueueStatus::Completed);

        let rest_entry_id = setup_spinning(&state).await;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/queue/{rest_entry_id}/complete"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 200);
        let entry = state
            .queue_service
            .get_by_id(rest_entry_id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(entry.status, QueueStatus::Completed);

        let reply = handle_message(
            &state,
            ClientMessage::Complete {
                entry_id: ws_entry_id,
            },
        )
        .await;
        assert!(matches!(
            reply,
            ServerMessage::CompleteErr { entry_id, error }
            if entry_id == ws_entry_id && !error.is_empty()
        ));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/queue/{ws_entry_id}/complete"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), 409);
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
