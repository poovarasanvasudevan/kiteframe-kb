use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use crate::{app::AppState, auth::Identity, error::AppError, shared::ApiResponse};

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct KnowledgeBase {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub visibility: String,
    pub status: String,
    pub language: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateKnowledgeBase {
    #[validate(length(min = 2, max = 120))]
    pub name: String,
    #[validate(length(min = 2, max = 140))]
    pub slug: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub visibility: Visibility,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateKnowledgeBase {
    #[validate(length(min = 2, max = 120))]
    pub name: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub visibility: Visibility,
    #[validate(length(min = 2, max = 12))]
    pub language: String,
    pub theme: Option<serde_json::Value>,
    pub seo_settings: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Visibility {
    Public,
    Private,
    Internal,
}

impl Visibility {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "PUBLIC",
            Self::Private => "PRIVATE",
            Self::Internal => "INTERNAL",
        }
    }
}

#[utoipa::path(get, path = "/api/v1/knowledge-bases", security(("bearer_auth" = [])), responses((status = 200, body = [KnowledgeBase])))]
pub async fn list(
    State(state): State<AppState>,
    identity: Identity,
) -> Result<Json<ApiResponse<Vec<KnowledgeBase>>>, AppError> {
    identity.require("kb.read")?;
    let records = sqlx::query_as::<_, KnowledgeBase>(
        "SELECT id, name, slug, description, visibility, status, language, created_at, updated_at FROM knowledge_bases WHERE organization_id = ? AND deleted_at IS NULL ORDER BY updated_at DESC"
    ).bind(identity.organization_id).fetch_all(&state.pool).await?;
    Ok(Json(ApiResponse::data(records)))
}

#[utoipa::path(post, path = "/api/v1/knowledge-bases", security(("bearer_auth" = [])), request_body = CreateKnowledgeBase, responses((status = 200, body = KnowledgeBase)))]
pub async fn create(
    State(state): State<AppState>,
    identity: Identity,
    Json(input): Json<CreateKnowledgeBase>,
) -> Result<Json<ApiResponse<KnowledgeBase>>, AppError> {
    identity.require("kb.create")?;
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    validate_slug(&input.slug)?;
    let id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO knowledge_bases (id, organization_id, name, slug, description, visibility, status, language, created_by) VALUES (?, ?, ?, ?, ?, ?, 'ACTIVE', 'en', ?)")
        .bind(id).bind(identity.organization_id).bind(input.name.trim()).bind(input.slug.trim().to_lowercase())
        .bind(input.description).bind(input.visibility.as_str()).bind(identity.user_id).execute(&state.pool).await;
    if let Err(sqlx::Error::Database(error)) = &result {
        if error.is_unique_violation() {
            return Err(AppError::Conflict(
                "A knowledge base already uses this slug".into(),
            ));
        }
    }
    result?;
    get_by_id(&state, identity.organization_id, id)
        .await
        .map(|record| Json(ApiResponse::data(record)))
}

#[utoipa::path(get, path = "/api/v1/knowledge-bases/{id}", params(("id" = Uuid, Path)), security(("bearer_auth" = [])), responses((status = 200, body = KnowledgeBase)))]
pub async fn get(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<KnowledgeBase>>, AppError> {
    identity.require("kb.read")?;
    Ok(Json(ApiResponse::data(
        get_by_id(&state, identity.organization_id, id).await?,
    )))
}

pub async fn update(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateKnowledgeBase>,
) -> Result<Json<ApiResponse<KnowledgeBase>>, AppError> {
    identity.require("kb.update")?;
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let changed = sqlx::query("UPDATE knowledge_bases SET name = ?, description = ?, visibility = ?, language = ?, theme = ?, seo_settings = ? WHERE id = ? AND organization_id = ? AND deleted_at IS NULL")
        .bind(input.name.trim()).bind(input.description).bind(input.visibility.as_str()).bind(input.language.to_lowercase()).bind(input.theme).bind(input.seo_settings).bind(id).bind(identity.organization_id).execute(&state.pool).await?.rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound("Knowledge base not found"));
    }
    Ok(Json(ApiResponse::data(
        get_by_id(&state, identity.organization_id, id).await?,
    )))
}

async fn get_by_id(
    state: &AppState,
    organization_id: Uuid,
    id: Uuid,
) -> Result<KnowledgeBase, AppError> {
    sqlx::query_as::<_, KnowledgeBase>("SELECT id, name, slug, description, visibility, status, language, created_at, updated_at FROM knowledge_bases WHERE id = ? AND organization_id = ? AND deleted_at IS NULL")
        .bind(id).bind(organization_id).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Knowledge base not found"))
}

fn validate_slug(slug: &str) -> Result<(), AppError> {
    let valid = !slug.is_empty()
        && slug.len() <= 140
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !slug.starts_with('-')
        && !slug.ends_with('-');
    valid.then_some(()).ok_or_else(|| {
        AppError::Validation(
            "Slug must contain lowercase letters, numbers, and single hyphens".into(),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn slug_rules_are_strict() {
        assert!(validate_slug("support-guides").is_ok());
        assert!(validate_slug("Support Guides").is_err());
        assert!(validate_slug("-support").is_err());
    }
}
