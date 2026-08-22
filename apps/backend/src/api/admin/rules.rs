use axum::Json;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::IntoParams;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

use crate::actions::action::ActionId;
use crate::error::RuleServiceError;
use crate::ingress::event::RuleTrigger;
use crate::rules::rule::{Rule, RuleConditions, RuleId};
use crate::state::AppState;

#[derive(Debug, Serialize, ToSchema)]
#[non_exhaustive]
pub struct RuleResponse {
    pub id: u32,
    pub name: String,
    pub enabled: bool,
    pub trigger: RuleTrigger,
    pub conditions: RuleConditions,
    pub action_id: u32,
    pub created_at: String,
    pub updated_at: String,
}

impl From<Rule> for RuleResponse {
    fn from(rule: Rule) -> Self {
        Self {
            id: rule.id.get(),
            name: rule.name,
            enabled: rule.enabled,
            trigger: rule.trigger,
            conditions: rule.conditions,
            action_id: rule.action_id.get(),
            created_at: rule.created_at.to_rfc3339(),
            updated_at: rule.updated_at.to_rfc3339(),
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[non_exhaustive]
pub struct UpsertRuleRequest {
    pub name: String,
    pub enabled: bool,
    pub trigger: RuleTrigger,
    pub conditions: RuleConditions,
    pub action_id: ActionId,
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[non_exhaustive]
pub struct RuleIdParam {
    pub id: u32,
}

#[utoipa::path(
    get,
    path = "/admin/rules",
    tag = "admin",
    responses(
        (status = 200, description = "List all rules", body = Vec<RuleResponse>),
    )
)]
pub async fn list_rules(
    State(state): State<AppState>,
) -> Result<Json<Vec<RuleResponse>>, RuleServiceError> {
    let rules = state.rule_service.list().await?;
    Ok(Json(rules.into_iter().map(RuleResponse::from).collect()))
}

#[utoipa::path(
    post,
    path = "/admin/rules",
    tag = "admin",
    request_body = UpsertRuleRequest,
    responses(
        (status = 201, description = "Rule created", body = RuleResponse),
        (status = 400, description = "Invalid conditions or action does not exist"),
    )
)]
pub async fn create_rule(
    State(state): State<AppState>,
    Json(body): Json<UpsertRuleRequest>,
) -> Result<(StatusCode, Json<RuleResponse>), RuleServiceError> {
    let rule = state
        .rule_service
        .create(
            &body.name,
            body.enabled,
            body.trigger,
            body.conditions,
            body.action_id,
        )
        .await?;
    Ok((StatusCode::CREATED, Json(RuleResponse::from(rule))))
}

#[utoipa::path(
    put,
    path = "/admin/rules/{id}",
    tag = "admin",
    params(RuleIdParam),
    request_body = UpsertRuleRequest,
    responses(
        (status = 200, description = "Rule updated", body = RuleResponse),
        (status = 404, description = "Rule not found"),
        (status = 400, description = "Invalid conditions or action does not exist"),
    )
)]
pub async fn update_rule(
    State(state): State<AppState>,
    Path(param): Path<RuleIdParam>,
    Json(body): Json<UpsertRuleRequest>,
) -> Result<Json<RuleResponse>, RuleServiceError> {
    let rule = Rule {
        id: RuleId::new(param.id),
        name: body.name,
        enabled: body.enabled,
        trigger: body.trigger,
        conditions: body.conditions,
        action_id: body.action_id,
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
    };
    state.rule_service.update(rule).await?;
    let updated = state.rule_service.get(RuleId::new(param.id)).await?;
    let updated = updated.ok_or(RuleServiceError::RuleNotFound)?;
    Ok(Json(RuleResponse::from(updated)))
}

#[utoipa::path(
    delete,
    path = "/admin/rules/{id}",
    tag = "admin",
    params(RuleIdParam),
    responses(
        (status = 204, description = "Rule removed"),
        (status = 404, description = "Rule not found"),
    )
)]
pub async fn delete_rule(
    State(state): State<AppState>,
    Path(param): Path<RuleIdParam>,
) -> Result<StatusCode, RuleServiceError> {
    state.rule_service.delete(RuleId::new(param.id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn session_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new().routes(routes!(list_rules))
}

pub fn root_router() -> OpenApiRouter<AppState> {
    OpenApiRouter::new()
        .routes(routes!(create_rule))
        .routes(routes!(update_rule, delete_rule))
}

#[cfg(test)]
mod tests {
    use axum::body::Body;
    use axum::body::to_bytes;
    use axum::http::header;
    use axum::http::{Request, StatusCode};
    use serde_json::Value;
    use tower::ServiceExt;

    use crate::actions::action::{ActionId, ActionKind};
    use crate::api::auth::SESSION_COOKIE;
    use crate::ingress::event::RuleTrigger;
    use crate::rules::rule::{MessageConditions, MessageMatcher, RuleConditions};
    use crate::state::AppState;
    use crate::test_fixtures::{api_path, session_cookie, test_router, test_state};

    fn chat_body(action_id: u32) -> String {
        format!(
            r#"{{"name":"spin","enabled":true,"trigger":"chat_message","conditions":{{"trigger":"chat_message","matcher":"contains","pattern":"!spin"}},"action_id":{action_id}}}"#
        )
    }

    async fn seed_action(state: &AppState, name: &str) -> u32 {
        state
            .action_service
            .create(name, ActionKind::EnqueueRoulette, true)
            .await
            .unwrap()
            .id
            .get()
    }

    #[tokio::test]
    async fn rules_require_session() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/rules"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn admin_can_list_rules() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        let action_id = seed_action(&state, "spin").await;
        state
            .rule_service
            .create(
                "spin-rule",
                true,
                RuleTrigger::ChatMessage,
                RuleConditions::ChatMessage(MessageConditions {
                    matcher: MessageMatcher::Contains,
                    pattern: Some("!spin".to_string()),
                }),
                ActionId::new(action_id),
            )
            .await
            .unwrap();

        let app = test_router(state.clone());
        let cookie = session_cookie(&state, "123").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/rules"))
                    .header(header::COOKIE, cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body.as_array().unwrap().len(), 1);
        assert_eq!(body[0]["name"], "spin-rule");
        assert_eq!(body[0]["trigger"], "chat_message");
    }

    #[tokio::test]
    async fn only_root_can_create_rule() {
        let state = test_state().await;
        state.admin_service.add("123", None).await.unwrap();
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());

        let user_cookie = session_cookie(&state, "123").await;
        let root_cookie = session_cookie(&state, "100").await;
        let action_id = seed_action(&state, "spin").await;

        let create = |cookie: String| {
            app.clone().oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/rules"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, cookie)
                    .body(Body::from(chat_body(action_id)))
                    .unwrap(),
            )
        };

        let response = create(user_cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::FORBIDDEN);

        let response = create(root_cookie).await.unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        assert_eq!(state.rule_service.list().await.unwrap().len(), 1);
    }

    #[tokio::test]
    async fn create_rule_validates_conditions_and_action() {
        let state = test_state().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let root_cookie = session_cookie(&state, "100").await;

        let invalid_conditions = r#"{"name":"spin","enabled":true,"trigger":"chat_message","conditions":{"trigger":"chat_message","matcher":"equals","pattern":null},"action_id":999}"#;
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/rules"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &root_cookie)
                    .body(Body::from(invalid_conditions))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);

        let unknown_action = r#"{"name":"spin","enabled":true,"trigger":"chat_message","conditions":{"trigger":"chat_message","matcher":"contains","pattern":"!spin"},"action_id":999}"#;
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(api_path("/admin/rules"))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, root_cookie)
                    .body(Body::from(unknown_action))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn root_can_update_and_delete_rule() {
        let state = test_state().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let root_cookie = session_cookie(&state, "100").await;
        let action_id = seed_action(&state, "spin").await;
        let rule = state
            .rule_service
            .create(
                "spin-rule",
                true,
                RuleTrigger::ChatMessage,
                RuleConditions::ChatMessage(MessageConditions {
                    matcher: MessageMatcher::Contains,
                    pattern: Some("!spin".to_string()),
                }),
                ActionId::new(action_id),
            )
            .await
            .unwrap();
        let rule_id = rule.id.get();

        let updated = format!(
            r#"{{"name":"renamed","enabled":false,"trigger":"chat_message","conditions":{{"trigger":"chat_message","matcher":"contains","pattern":"!spin"}},"action_id":{action_id}}}"#
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri(api_path(&format!("/admin/rules/{rule_id}")))
                    .header(header::CONTENT_TYPE, "application/json")
                    .header(header::COOKIE, &root_cookie)
                    .body(Body::from(updated))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap())
                .unwrap();
        assert_eq!(body["name"], "renamed");
        assert_eq!(body["enabled"], false);

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path(&format!("/admin/rules/{rule_id}")))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NO_CONTENT);
        assert!(state.rule_service.get(rule.id).await.unwrap().is_none());
    }

    #[tokio::test]
    async fn delete_missing_rule_is_not_found() {
        let state = test_state().await;
        state.admin_service.add("100", None).await.unwrap();
        state.admin_service.set_root("100", true).await.unwrap();
        let app = test_router(state.clone());
        let root_cookie = session_cookie(&state, "100").await;

        let response = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri(api_path("/admin/rules/999"))
                    .header(header::COOKIE, root_cookie)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn rules_require_admin_session_cookie() {
        let state = test_state().await;
        let app = test_router(state.clone());

        let response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri(api_path("/admin/rules"))
                    .header(header::COOKIE, format!("{SESSION_COOKIE}=bogus"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
