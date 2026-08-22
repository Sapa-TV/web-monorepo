use std::collections::HashSet;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use serde::Serialize;
use twitch_api::helix::points::CustomReward;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RewardResponse {
    pub id: String,
    pub title: String,
    pub cost: u64,
    pub is_enabled: bool,
    pub is_paused: bool,
    pub used_in_rules: bool,
}

impl RewardResponse {
    fn build(reward: CustomReward, used_in_rules: &HashSet<String>) -> Self {
        Self {
            id: reward.id.to_string(),
            title: reward.title,
            cost: reward.cost as u64,
            is_enabled: reward.is_enabled,
            is_paused: reward.is_paused,
            used_in_rules: used_in_rules.contains(&reward.id.to_string()),
        }
    }
}

#[utoipa::path(
    get,
    path = "/admin/rewards",
    tag = "admin",
    responses(
        (status = 200, description = "List custom rewards from Twitch", body = Vec<RewardResponse>),
        (status = 400, description = "Twitch is not configured"),
        (status = 401, description = "No valid user token"),
        (status = 502, description = "Twitch API request failed"),
    )
)]
pub async fn list_rewards(
    State(state): State<AppState>,
) -> Result<Json<Vec<RewardResponse>>, StatusCode> {
    let twitch = state.twitch_api.as_ref().ok_or(StatusCode::BAD_REQUEST)?;
    let rewards = twitch.custom_rewards().await.map_err(StatusCode::from)?;
    let used = state
        .rule_service
        .referenced_reward_ids()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rewards
            .into_iter()
            .map(|reward| RewardResponse::build(reward, &used))
            .collect(),
    ))
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_rewards))
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use axum::body::Body;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::config::runtime::RuntimeConfig;
    use crate::config::static_config::StaticConfig;
    use crate::config::store::ConfigStore;
    use crate::config::twitch::TwitchConfig;
    use crate::db::inmemory_config::InMemoryConfigRepository;
    use crate::db::inmemory_platform_credential::InMemoryPlatformCredentialRepository;
    use crate::random::StandartRandomProvider;
    use crate::state::{AppState, AppStateBuilder};
    use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

    fn twitch_config() -> Arc<TwitchConfig> {
        Arc::new(TwitchConfig {
            client_id: "cid".to_string(),
            client_secret: "cs".to_string(),
            broadcaster_id: "bc".to_string(),
            redirect_uri: "https://localhost/cb".to_string(),
            credentials_redirect_uri: "https://localhost/creds/cb".to_string(),
            csrf_ttl_secs: 600,
        })
    }

    async fn state_with_twitch() -> AppState {
        let static_cfg = StaticConfig {
            twitch: Some(twitch_config()),
            ..StaticConfig::default()
        };
        let config_store = Arc::new(ConfigStore::new(
            Arc::new(static_cfg),
            RuntimeConfig::test_runtime("test-key"),
            Arc::new(InMemoryConfigRepository::new()),
        ));
        AppStateBuilder::new(
            StandartRandomProvider,
            config_store,
            Arc::new(InMemoryPlatformCredentialRepository::new()),
        )
        .with_empty_repos()
        .build()
        .await
        .expect("failed to build test state")
    }

    #[tokio::test]
    async fn rewards_require_session() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/rewards"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn rewards_without_twitch_config_is_bad_request() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/rewards"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn rewards_without_credentials_is_unauthorized() {
        let state = state_with_twitch().await;
        state.admin_service.add("123", None).await.unwrap();
        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/rewards"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
