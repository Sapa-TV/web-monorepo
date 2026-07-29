use std::convert::Infallible;
use std::sync::Arc;

use axum::extract::State;
use axum::response::sse::{Event, KeepAlive, Sse};
use tokio_stream::StreamExt;
use tokio_stream::wrappers::BroadcastStream;

use crate::queue::events::SpinEvent;
use crate::state::AppState;

#[utoipa::path(
    get,
    path = "/api/events",
    tag = "events",
    responses(
        (status = 200, description = "Server-Sent Events stream", content_type = "text/event-stream"),
    )
)]
pub async fn events_handler(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.event_publisher.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| match result {
        Ok(event) => Some(Ok(spin_event_to_sse(event))),
        Err(_) => None,
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

fn spin_event_to_sse(event: Arc<SpinEvent>) -> Event {
    match event.as_ref() {
        SpinEvent::Started {
            entry_id,
            slot_name,
            slot_rarity,
            user_name,
        } => Event::default()
            .event("spin_started")
            .json_data(serde_json::json!({
                "entry_id": entry_id.value(),
                "slot_name": slot_name,
                "slot_rarity": slot_rarity,
                "user_name": user_name,
            }))
            .unwrap_or_else(|_| Event::default().data("")),
        SpinEvent::Completed { entry_id } => Event::default()
            .event("spin_completed")
            .json_data(serde_json::json!({
                "entry_id": entry_id.value(),
            }))
            .unwrap_or_else(|_| Event::default().data("")),
        SpinEvent::Error { entry_id } => Event::default()
            .event("spin_error")
            .json_data(serde_json::json!({
                "entry_id": entry_id.value(),
            }))
            .unwrap_or_else(|_| Event::default().data("")),
    }
}

pub fn router() -> axum::Router<AppState> {
    use axum::routing::get;
    axum::Router::new().route("/api/events", get(events_handler))
}
