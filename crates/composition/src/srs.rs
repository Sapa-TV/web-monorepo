use std::sync::Arc;

use model::consts::stream::{SRS_APP, SRS_POLL_INTERVAL, SRS_POLL_TIMEOUT, SRS_STREAM};
use serde::Deserialize;
use subtle::ConstantTimeEq;
use tokio::select;
use tokio::time;
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;

use crate::stream::StreamStatus;

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct SrsHookPayload {
    pub action: String,
    pub app: String,
    pub stream: String,
    #[serde(default)]
    pub param: String,
}

#[derive(Debug)]
#[non_exhaustive]
pub struct SrsHookOutcome {
    pub status: u16,
    pub code: i32,
    pub message: &'static str,
}

impl SrsHookOutcome {
    fn accepted() -> Self {
        Self {
            status: 200,
            code: 0,
            message: "ok",
        }
    }

    fn unauthorized() -> Self {
        Self {
            status: 401,
            code: -1,
            message: "invalid token",
        }
    }

    fn denied() -> Self {
        Self {
            status: 200,
            code: 1,
            message: "publish denied",
        }
    }
}

pub fn handle_hook(
    status: &StreamStatus,
    expected_token: Option<&str>,
    token: &str,
    expected_stream_key: Option<&str>,
    payload: &SrsHookPayload,
) -> SrsHookOutcome {
    if !token_matches(expected_token, token) {
        return SrsHookOutcome::unauthorized();
    }
    let tracked = payload.app == SRS_APP && payload.stream == SRS_STREAM;
    if payload.action == "on_publish"
        && let Some(key) = expected_stream_key
        && (!tracked || !publish_key_matches(&payload.param, key))
    {
        return SrsHookOutcome::denied();
    }
    if tracked {
        match payload.action.as_str() {
            "on_publish" => status.set_rtmp_online(true),
            "on_unpublish" => status.set_rtmp_online(false),
            _ => {}
        }
    }
    SrsHookOutcome::accepted()
}

fn publish_key_matches(param: &str, expected: &str) -> bool {
    param
        .trim_start_matches('?')
        .split('&')
        .filter_map(|pair| pair.split_once('='))
        .any(|(name, value)| {
            name == "key" && bool::from(value.as_bytes().ct_eq(expected.as_bytes()))
        })
}

fn token_matches(expected: Option<&str>, given: &str) -> bool {
    expected.is_some_and(|expected| expected.as_bytes().ct_eq(given.as_bytes()).into())
}

#[derive(Debug, Deserialize)]
struct SrsStreamsResponse {
    #[serde(default)]
    streams: Vec<SrsStream>,
}

#[derive(Debug, Deserialize)]
struct SrsStream {
    name: String,
    app: String,
    #[serde(default)]
    publish: Option<SrsPublish>,
}

#[derive(Debug, Deserialize)]
struct SrsPublish {
    #[serde(default)]
    active: bool,
}

fn streams_online(body: &SrsStreamsResponse) -> bool {
    body.streams.iter().any(|stream| {
        stream.app == SRS_APP
            && stream.name == SRS_STREAM
            && stream.publish.as_ref().is_none_or(|publish| publish.active)
    })
}

pub async fn fetch_online(client: &reqwest::Client, api_url: &str) -> Result<bool, reqwest::Error> {
    let url = format!("{}/api/v1/streams/", api_url.trim_end_matches('/'));
    let body: SrsStreamsResponse = client
        .get(url)
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    Ok(streams_online(&body))
}

pub async fn stream_status_task(
    status: Arc<StreamStatus>,
    api_url: String,
    shutdown: CancellationToken,
) {
    let client = reqwest::Client::builder()
        .timeout(SRS_POLL_TIMEOUT)
        .build()
        .expect("failed to build SRS http client");
    let mut first = true;
    loop {
        match fetch_online(&client, &api_url).await {
            Ok(online) => status.set_rtmp_online(online),
            Err(e) => tracing::warn!("srs stream poll failed: {e}"),
        }
        if first {
            first = false;
            status.mark_ready();
        }
        select! {
            biased;
            _ = shutdown.cancelled() => break,
            _ = time::sleep(SRS_POLL_INTERVAL) => {},
        }
    }
}

#[cfg(test)]
#[path = "srs.test.rs"]
mod tests;
