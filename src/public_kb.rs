use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use crate::{app::AppState, error::AppError, shared::ApiResponse};

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct PublicKnowledgeBase {
    pub id: Uuid,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub language: String,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct PublicArticle {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub content: String,
    pub category_name: String,
    pub published_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct SearchParams {
    #[validate(length(min = 1, max = 200))]
    pub q: String,
    pub limit: Option<u32>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct FeedbackInput {
    pub helpful: bool,
    #[validate(length(max = 2000))]
    pub comment: Option<String>,
    #[validate(length(max = 128))]
    pub visitor_id: Option<String>,
}

pub async fn knowledge_base(
    State(state): State<AppState>,
    Path(kb_slug): Path<String>,
) -> Result<Json<ApiResponse<PublicKnowledgeBase>>, AppError> {
    let kb = sqlx::query_as::<_, PublicKnowledgeBase>("SELECT id, name, slug, description, language FROM knowledge_bases WHERE slug = ? AND visibility = 'PUBLIC' AND status = 'ACTIVE' AND deleted_at IS NULL")
        .bind(kb_slug).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Knowledge base not found"))?;
    Ok(Json(ApiResponse::data(kb)))
}

pub async fn article(
    State(state): State<AppState>,
    Path((kb_slug, article_slug)): Path<(String, String)>,
) -> Result<Json<ApiResponse<PublicArticle>>, AppError> {
    let article = sqlx::query_as::<_, PublicArticle>("SELECT a.id, v.title, a.slug, JSON_UNQUOTE(JSON_EXTRACT(v.metadata, '$.short_description')) AS short_description, v.content, c.name AS category_name, a.published_at, a.updated_at FROM articles a JOIN knowledge_bases kb ON kb.id = a.knowledge_base_id JOIN categories c ON c.id = a.category_id JOIN article_versions v ON v.id = a.published_version_id WHERE kb.slug = ? AND kb.visibility = 'PUBLIC' AND kb.status = 'ACTIVE' AND a.slug = ? AND a.status = 'PUBLISHED' AND a.visibility = 'PUBLIC' AND c.visibility = 'PUBLIC' AND a.deleted_at IS NULL")
        .bind(kb_slug).bind(article_slug).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Article not found"))?;
    sqlx::query("INSERT INTO article_views (organization_id, article_id) SELECT organization_id, id FROM articles WHERE id = ? AND status = 'PUBLISHED'")
        .bind(article.id).execute(&state.pool).await?;
    Ok(Json(ApiResponse::data(article)))
}

pub async fn search(
    State(state): State<AppState>,
    Path(kb_slug): Path<String>,
    Query(params): Query<SearchParams>,
) -> Result<Json<ApiResponse<Vec<PublicArticle>>>, AppError> {
    params
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let limit = params.limit.unwrap_or(20).clamp(1, 50);
    let kb = sqlx::query_as::<_, (Uuid, Uuid)>("SELECT id, organization_id FROM knowledge_bases WHERE slug = ? AND visibility = 'PUBLIC' AND status = 'ACTIVE' AND deleted_at IS NULL")
        .bind(kb_slug).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Knowledge base not found"))?;
    let records = sqlx::query_as::<_, PublicArticle>("SELECT a.id, v.title, a.slug, JSON_UNQUOTE(JSON_EXTRACT(v.metadata, '$.short_description')) AS short_description, v.content, c.name AS category_name, a.published_at, a.updated_at FROM articles a JOIN categories c ON c.id = a.category_id JOIN article_versions v ON v.id = a.published_version_id WHERE a.organization_id = ? AND a.knowledge_base_id = ? AND a.status = 'PUBLISHED' AND a.visibility = 'PUBLIC' AND c.visibility = 'PUBLIC' AND a.deleted_at IS NULL AND (MATCH(a.title, a.short_description, a.draft_content) AGAINST (? IN NATURAL LANGUAGE MODE) OR a.title LIKE CONCAT(?, '%')) ORDER BY MATCH(a.title, a.short_description, a.draft_content) AGAINST (? IN NATURAL LANGUAGE MODE) DESC, a.updated_at DESC LIMIT ?")
        .bind(kb.1).bind(kb.0).bind(&params.q).bind(&params.q).bind(&params.q).bind(limit).fetch_all(&state.pool).await?;
    sqlx::query("INSERT INTO search_queries (organization_id, knowledge_base_id, query, results_count) VALUES (?, ?, ?, ?)")
        .bind(kb.1).bind(kb.0).bind(params.q).bind(records.len() as u32).execute(&state.pool).await?;
    Ok(Json(ApiResponse::data(records)))
}

pub async fn feedback(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
    Json(input): Json<FeedbackInput>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let article = sqlx::query_as::<_, (Uuid, Uuid)>("SELECT id, organization_id FROM articles WHERE id = ? AND status = 'PUBLISHED' AND visibility = 'PUBLIC' AND deleted_at IS NULL")
        .bind(id).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Article not found"))?;
    let visitor_hash = input.visitor_id.map(|visitor| {
        use std::hash::{Hash, Hasher};
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        visitor.hash(&mut hasher);
        hasher.finish().to_be_bytes().to_vec()
    });
    if let Some(hash) = &visitor_hash {
        let recent = sqlx::query_scalar::<_, i8>("SELECT 1 FROM article_feedback WHERE article_id = ? AND visitor_hash = ? AND created_at > DATE_SUB(CURRENT_TIMESTAMP, INTERVAL 1 DAY) LIMIT 1")
            .bind(id).bind(hash).fetch_optional(&state.pool).await?;
        if recent.is_some() {
            return Err(AppError::Conflict(
                "Feedback was already recorded for this article".into(),
            ));
        }
    }
    sqlx::query("INSERT INTO article_feedback (id, organization_id, article_id, helpful, comment, visitor_hash) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4()).bind(article.1).bind(article.0).bind(input.helpful).bind(input.comment).bind(visitor_hash).execute(&state.pool).await?;
    Ok(Json(ApiResponse::data(
        serde_json::json!({"recorded": true}),
    )))
}
