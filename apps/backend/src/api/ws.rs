use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use tokio::sync::broadcast;

use crate::state::AppState;

pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.event_publisher.subscribe();

    tracing::info!("ws client connected");

    loop {
        better_tokio_select::tokio_select!(match .. {
            .. if let result = rx.recv() => match result {
                Ok(event) => {
                    let Ok(json) = serde_json::to_string(&*event) else { break };
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
                Some(Ok(Message::Close(_))) | None => break,
                Some(Err(_)) => break,
                _ => {}
            },
        })
    }

    tracing::debug!("ws client disconnected");
}

pub fn router() -> axum::Router<AppState> {
    axum::Router::new().route("/ws", axum::routing::get(ws_handler))
}
