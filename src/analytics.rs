use axum::{Json, extract::State};
use serde::Serialize;
use sqlx::FromRow;

use crate::{app::AppState, auth::Identity, error::AppError, shared::ApiResponse};

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct AnalyticsSummary {
    pub total_articles: i64,
    pub published_articles: i64,
    pub draft_articles: i64,
    pub pending_reviews: i64,
    pub article_views: i64,
    pub searches: i64,
    pub failed_searches: i64,
    pub helpful_votes: i64,
    pub unhelpful_votes: i64,
}

pub async fn summary(
    State(state): State<AppState>,
    identity: Identity,
) -> Result<Json<ApiResponse<AnalyticsSummary>>, AppError> {
    identity.require("analytics.read")?;
    let summary = sqlx::query_as::<_, AnalyticsSummary>("SELECT (SELECT COUNT(*) FROM articles WHERE organization_id = ? AND deleted_at IS NULL) total_articles, (SELECT COUNT(*) FROM articles WHERE organization_id = ? AND status = 'PUBLISHED' AND deleted_at IS NULL) published_articles, (SELECT COUNT(*) FROM articles WHERE organization_id = ? AND status = 'DRAFT' AND deleted_at IS NULL) draft_articles, (SELECT COUNT(*) FROM article_reviews WHERE organization_id = ? AND status = 'PENDING') pending_reviews, (SELECT COUNT(*) FROM article_views WHERE organization_id = ?) article_views, (SELECT COUNT(*) FROM search_queries WHERE organization_id = ?) searches, (SELECT COUNT(*) FROM search_queries WHERE organization_id = ? AND results_count = 0) failed_searches, (SELECT COUNT(*) FROM article_feedback WHERE organization_id = ? AND helpful = TRUE) helpful_votes, (SELECT COUNT(*) FROM article_feedback WHERE organization_id = ? AND helpful = FALSE) unhelpful_votes")
        .bind(identity.organization_id).bind(identity.organization_id).bind(identity.organization_id).bind(identity.organization_id)
        .bind(identity.organization_id).bind(identity.organization_id).bind(identity.organization_id).bind(identity.organization_id).bind(identity.organization_id)
        .fetch_one(&state.pool).await?;
    Ok(Json(ApiResponse::data(summary)))
}
