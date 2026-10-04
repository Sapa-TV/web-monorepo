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
    #[serde(skip)]
    _sealed: (),
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
            _sealed: (),
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
    #[serde(skip)]
    _sealed: (),
}

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
#[non_exhaustive]
pub struct RuleIdParam {
    pub id: u32,
    #[serde(skip)]
    _sealed: (),
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
    let rule = Rule::new(
        RuleId::new(param.id),
        body.name,
        body.enabled,
        body.trigger,
        body.conditions,
        body.action_id,
        chrono::Utc::now(),
        chrono::Utc::now(),
    );
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
#[path = "rules.test.rs"]
mod tests;
