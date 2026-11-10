use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use domain::push::service::{PushOutcome, PushPayload, PushSender};
use model::push::PushSubscription;
use web_push_native::jwt_simple::algorithms::ES256KeyPair;
use web_push_native::p256::PublicKey;
use web_push_native::{Auth, WebPushBuilder};

#[non_exhaustive]
pub struct WebPushSender {
    client: reqwest::Client,
    key_pair: Option<ES256KeyPair>,
    subject: String,
}

impl WebPushSender {
    pub fn new(private_key: Option<&str>, subject: &str) -> Self {
        let key_pair = private_key.and_then(|encoded| {
            let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|error| {
                tracing::error!("invalid VAPID private key encoding: {error}");
            });
            bytes.ok().and_then(|bytes| {
                ES256KeyPair::from_bytes(&bytes)
                    .map_err(|error| {
                        tracing::error!("invalid VAPID private key: {error:?}");
                    })
                    .ok()
            })
        });
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .unwrap_or_default(),
            key_pair,
            subject: subject.to_string(),
        }
    }
}

impl PushSender for WebPushSender {
    async fn send(
        &self,
        subscription: &PushSubscription,
        payload: &PushPayload,
    ) -> Result<PushOutcome, String> {
        let key_pair = self
            .key_pair
            .as_ref()
            .ok_or_else(|| "VAPID private key is not configured".to_string())?;
        let endpoint = subscription
            .endpoint
            .parse()
            .map_err(|error| format!("invalid endpoint: {error:?}"))?;
        let p256dh = URL_SAFE_NO_PAD
            .decode(&subscription.p256dh)
            .map_err(|error| format!("invalid p256dh: {error}"))?;
        let auth = URL_SAFE_NO_PAD
            .decode(&subscription.auth)
            .map_err(|error| format!("invalid auth: {error}"))?;
        if auth.len() != 16 {
            return Err("auth must be 16 bytes".to_string());
        }
        let public_key = PublicKey::from_sec1_bytes(&p256dh)
            .map_err(|error| format!("invalid p256dh point: {error:?}"))?;
        let body = serde_json::to_vec(payload).map_err(|error| error.to_string())?;
        let mut auth_bytes = Auth::default();
        auth_bytes.copy_from_slice(&auth);
        let request = WebPushBuilder::new(endpoint, public_key, auth_bytes)
            .with_vapid(key_pair, &self.subject)
            .build(body)
            .map_err(|error| error.to_string())?;

        let method = reqwest::Method::from_bytes(request.method().as_str().as_bytes())
            .map_err(|error| error.to_string())?;
        let url = request.uri().to_string();
        let mut outgoing = self.client.request(method, url);
        for (name, value) in request.headers() {
            outgoing = outgoing.header(name, value);
        }
        let response = outgoing
            .body(request.into_body())
            .send()
            .await
            .map_err(|error| error.to_string())?;
        let status = response.status();
        let outcome = match status.as_u16() {
            200..=299 => PushOutcome::Delivered,
            404 | 410 => PushOutcome::Gone,
            429 => PushOutcome::Retry,
            500..=599 => PushOutcome::Retry,
            _ => PushOutcome::Gone,
        };
        if outcome != PushOutcome::Delivered {
            tracing::warn!(
                subscription_id = subscription.id.get(),
                status = status.as_u16(),
                "push endpoint rejected the notification"
            );
        }
        Ok(outcome)
    }
}
