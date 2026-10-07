use axum::{
    Json,
    extract::{Path, Query, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySql, Transaction};
use uuid::Uuid;
use validator::Validate;

use crate::{app::AppState, auth::Identity, error::AppError, shared::ApiResponse};

#[derive(Debug, Clone, Serialize, FromRow, utoipa::ToSchema)]
pub struct Article {
    pub id: Uuid,
    pub knowledge_base_id: Uuid,
    pub category_id: Uuid,
    pub author_id: Uuid,
    pub current_version_id: Option<Uuid>,
    pub title: String,
    pub slug: String,
    pub short_description: Option<String>,
    pub draft_content: String,
    pub status: String,
    pub visibility: String,
    pub language: String,
    pub featured: bool,
    pub pinned: bool,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub canonical_url: Option<String>,
    pub published_at: Option<DateTime<Utc>>,
    pub scheduled_publish_at: Option<DateTime<Utc>>,
    pub lock_version: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct ArticleVersion {
    pub id: Uuid,
    pub article_id: Uuid,
    pub version_number: u32,
    pub title: String,
    pub content: String,
    pub metadata: serde_json::Value,
    pub author_id: Uuid,
    pub change_summary: Option<String>,
    pub is_published: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateArticle {
    pub knowledge_base_id: Uuid,
    pub category_id: Uuid,
    #[validate(length(min = 2, max = 240))]
    pub title: String,
    #[validate(length(min = 2, max = 260))]
    pub slug: String,
    #[validate(length(max = 500))]
    pub short_description: Option<String>,
    pub content: String,
    pub visibility: ArticleVisibility,
    #[validate(length(max = 500))]
    pub change_summary: Option<String>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateArticle {
    #[validate(length(min = 2, max = 240))]
    pub title: String,
    #[validate(length(max = 500))]
    pub short_description: Option<String>,
    pub content: String,
    pub category_id: Uuid,
    pub visibility: ArticleVisibility,
    pub lock_version: u64,
    #[validate(length(max = 500))]
    pub change_summary: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ArticleVisibility {
    Public,
    Private,
    Internal,
}
impl ArticleVisibility {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Public => "PUBLIC",
            Self::Private => "PRIVATE",
            Self::Internal => "INTERNAL",
        }
    }
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct DecisionInput {
    #[validate(length(min = 2, max = 2000))]
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ArticleFilters {
    pub status: Option<String>,
    pub knowledge_base_id: Option<Uuid>,
    pub query: Option<String>,
    pub page: Option<u32>,
    pub page_size: Option<u32>,
}

#[utoipa::path(get, path = "/api/v1/articles", security(("bearer_auth" = [])), responses((status = 200, body = [Article])))]
pub async fn list(
    State(state): State<AppState>,
    identity: Identity,
    Query(filters): Query<ArticleFilters>,
) -> Result<Json<ApiResponse<Vec<Article>>>, AppError> {
    identity.require("article.read")?;
    let size = filters.page_size.unwrap_or(20).clamp(1, 100);
    let offset = filters.page.unwrap_or(1).saturating_sub(1) * size;
    let records = sqlx::query_as::<_, Article>(&format!(
        "{} AND (? IS NULL OR status = ?) AND (? IS NULL OR knowledge_base_id = ?) AND (? IS NULL OR title LIKE CONCAT('%', ?, '%')) ORDER BY updated_at DESC LIMIT ? OFFSET ?",
        article_select("organization_id = ? AND deleted_at IS NULL")
    ))
    .bind(identity.organization_id).bind(&filters.status).bind(&filters.status)
    .bind(filters.knowledge_base_id).bind(filters.knowledge_base_id)
    .bind(&filters.query).bind(&filters.query).bind(size).bind(offset)
    .fetch_all(&state.pool).await?;
    Ok(Json(ApiResponse::data(records)))
}

#[utoipa::path(post, path = "/api/v1/articles", request_body = CreateArticle, security(("bearer_auth" = [])), responses((status = 200, body = Article)))]
pub async fn create(
    State(state): State<AppState>,
    identity: Identity,
    Json(input): Json<CreateArticle>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.create")?;
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    validate_slug(&input.slug)?;
    let safe_content = ammonia::clean(&input.content);
    let mut transaction = state.pool.begin().await?;
    ensure_category(
        &mut transaction,
        identity.organization_id,
        input.knowledge_base_id,
        input.category_id,
    )
    .await?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO articles (id, organization_id, knowledge_base_id, category_id, author_id, title, slug, short_description, draft_content, status, visibility, language) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'DRAFT', ?, 'en')")
        .bind(id).bind(identity.organization_id).bind(input.knowledge_base_id).bind(input.category_id).bind(identity.user_id)
        .bind(input.title.trim()).bind(input.slug.trim().to_lowercase()).bind(input.short_description).bind(safe_content).bind(input.visibility.as_str())
        .execute(&mut *transaction).await.map_err(map_unique)?;
    let version = snapshot(
        &mut transaction,
        identity.organization_id,
        id,
        identity.user_id,
        input.change_summary,
    )
    .await?;
    sqlx::query("UPDATE articles SET current_version_id = ? WHERE id = ?")
        .bind(version)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    audit(
        &mut transaction,
        &identity,
        "ARTICLE_CREATED",
        "article",
        id,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(ApiResponse::data(
        get_scoped(&state, identity.organization_id, id).await?,
    )))
}

#[utoipa::path(get, path = "/api/v1/articles/{id}", params(("id" = Uuid, Path)), security(("bearer_auth" = [])), responses((status = 200, body = Article)))]
pub async fn get(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.read")?;
    Ok(Json(ApiResponse::data(
        get_scoped(&state, identity.organization_id, id).await?,
    )))
}

#[utoipa::path(patch, path = "/api/v1/articles/{id}", params(("id" = Uuid, Path)), request_body = UpdateArticle, security(("bearer_auth" = [])), responses((status = 200, body = Article)))]
pub async fn update(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateArticle>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.update")?;
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let safe_content = ammonia::clean(&input.content);
    let mut transaction = state.pool.begin().await?;
    let article = lock_article(&mut transaction, identity.organization_id, id).await?;
    ensure_editable(&article.status)?;
    ensure_category(
        &mut transaction,
        identity.organization_id,
        article.knowledge_base_id,
        input.category_id,
    )
    .await?;
    let changed = sqlx::query("UPDATE articles SET title = ?, short_description = ?, draft_content = ?, category_id = ?, visibility = ?, lock_version = lock_version + 1 WHERE id = ? AND organization_id = ? AND lock_version = ?")
        .bind(input.title.trim()).bind(input.short_description).bind(safe_content).bind(input.category_id).bind(input.visibility.as_str())
        .bind(id).bind(identity.organization_id).bind(input.lock_version).execute(&mut *transaction).await?.rows_affected();
    if changed == 0 {
        return Err(AppError::Conflict(
            "This article changed in another session. Reload before saving.".into(),
        ));
    }
    let version = snapshot(
        &mut transaction,
        identity.organization_id,
        id,
        identity.user_id,
        input.change_summary,
    )
    .await?;
    sqlx::query("UPDATE articles SET current_version_id = ? WHERE id = ?")
        .bind(version)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    audit(
        &mut transaction,
        &identity,
        "ARTICLE_UPDATED",
        "article",
        id,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(ApiResponse::data(
        get_scoped(&state, identity.organization_id, id).await?,
    )))
}

#[utoipa::path(get, path = "/api/v1/articles/{id}/versions", params(("id" = Uuid, Path)), security(("bearer_auth" = [])), responses((status = 200, body = [ArticleVersion])))]
pub async fn versions(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<ArticleVersion>>>, AppError> {
    identity.require("article.read")?;
    get_scoped(&state, identity.organization_id, id).await?;
    let records = sqlx::query_as::<_, ArticleVersion>("SELECT id, article_id, version_number, title, content, metadata, author_id, change_summary, is_published, created_at FROM article_versions WHERE article_id = ? AND organization_id = ? ORDER BY version_number DESC")
        .bind(id).bind(identity.organization_id).fetch_all(&state.pool).await?;
    Ok(Json(ApiResponse::data(records)))
}

pub async fn submit_review(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.update")?;
    transition_with_review(
        &state,
        &identity,
        id,
        "DRAFT",
        "IN_REVIEW",
        "ARTICLE_SUBMITTED",
    )
    .await?;
    Ok(Json(ApiResponse::data(
        get_scoped(&state, identity.organization_id, id).await?,
    )))
}

pub async fn approve(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
    Json(input): Json<DecisionInput>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.review")?;
    decide_review(
        &state,
        &identity,
        id,
        "APPROVED",
        "APPROVED",
        input.comment,
        "ARTICLE_APPROVED",
    )
    .await?;
    Ok(Json(ApiResponse::data(
        get_scoped(&state, identity.organization_id, id).await?,
    )))
}

pub async fn reject(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
    Json(input): Json<DecisionInput>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.review")?;
    let comment = input
        .comment
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            AppError::Validation("A reason is required when requesting changes".into())
        })?;
    decide_review(
        &state,
        &identity,
        id,
        "DRAFT",
        "CHANGES_REQUESTED",
        Some(comment),
        "ARTICLE_REJECTED",
    )
    .await?;
    Ok(Json(ApiResponse::data(
        get_scoped(&state, identity.organization_id, id).await?,
    )))
}

pub async fn publish(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.publish")?;
    let mut transaction = state.pool.begin().await?;
    let article = lock_article(&mut transaction, identity.organization_id, id).await?;
    transition_allowed(&article.status, "PUBLISHED")?;
    let version_id = article
        .current_version_id
        .ok_or_else(|| AppError::Conflict("Article has no version to publish".into()))?;
    sqlx::query("UPDATE article_versions SET is_published = TRUE WHERE id = ? AND article_id = ?")
        .bind(version_id)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    sqlx::query("UPDATE articles SET status = 'PUBLISHED', published_version_id = ?, published_at = COALESCE(published_at, CURRENT_TIMESTAMP(6)) WHERE id = ?").bind(version_id).bind(id).execute(&mut *transaction).await?;
    audit(
        &mut transaction,
        &identity,
        "ARTICLE_PUBLISHED",
        "article",
        id,
    )
    .await?;
    transaction.commit().await?;
    Ok(Json(ApiResponse::data(
        get_scoped(&state, identity.organization_id, id).await?,
    )))
}

pub async fn archive(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.archive")?;
    simple_transition(&state, &identity, id, "ARCHIVED", "ARTICLE_ARCHIVED").await
}

pub async fn restore(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    identity.require("article.restore")?;
    simple_transition(&state, &identity, id, "DRAFT", "ARTICLE_RESTORED").await
}

async fn simple_transition(
    state: &AppState,
    identity: &Identity,
    id: Uuid,
    target: &str,
    action: &str,
) -> Result<Json<ApiResponse<Article>>, AppError> {
    let mut transaction = state.pool.begin().await?;
    let article = lock_article(&mut transaction, identity.organization_id, id).await?;
    transition_allowed(&article.status, target)?;
    sqlx::query("UPDATE articles SET status = ? WHERE id = ?")
        .bind(target)
        .bind(id)
        .execute(&mut *transaction)
        .await?;
    audit(&mut transaction, identity, action, "article", id).await?;
    transaction.commit().await?;
    Ok(Json(ApiResponse::data(
        get_scoped(state, identity.organization_id, id).await?,
    )))
}

async fn transition_with_review(
    state: &AppState,
    identity: &Identity,
    id: Uuid,
    from: &str,
    target: &str,
    action: &str,
) -> Result<(), AppError> {
    let mut tx = state.pool.begin().await?;
    let article = lock_article(&mut tx, identity.organization_id, id).await?;
    if article.status != from {
        return Err(AppError::Conflict(format!(
            "Article must be {from} before this action"
        )));
    }
    let version_id = article
        .current_version_id
        .ok_or_else(|| AppError::Conflict("Article has no current version".into()))?;
    sqlx::query("UPDATE articles SET status = ? WHERE id = ?")
        .bind(target)
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO article_reviews (id, organization_id, article_id, version_id, requested_by) VALUES (?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4()).bind(identity.organization_id).bind(id).bind(version_id).bind(identity.user_id).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO notifications (id, organization_id, user_id, event_type, title, body, resource_type, resource_id) SELECT UUID_TO_BIN(UUID()), ?, ur.user_id, 'ARTICLE_SUBMITTED', 'Article ready for review', ?, 'article', ? FROM user_roles ur JOIN role_permissions rp ON rp.role_id = ur.role_id JOIN permissions p ON p.id = rp.permission_id JOIN users u ON u.id = ur.user_id WHERE p.name = 'article.review' AND u.organization_id = ? GROUP BY ur.user_id")
        .bind(identity.organization_id).bind(&article.title).bind(id).bind(identity.organization_id).execute(&mut *tx).await?;
    audit(&mut tx, identity, action, "article", id).await?;
    tx.commit().await?;
    Ok(())
}

async fn decide_review(
    state: &AppState,
    identity: &Identity,
    id: Uuid,
    article_status: &str,
    review_status: &str,
    comment: Option<String>,
    action: &str,
) -> Result<(), AppError> {
    let mut tx = state.pool.begin().await?;
    let article = lock_article(&mut tx, identity.organization_id, id).await?;
    if article.status != "IN_REVIEW" {
        return Err(AppError::Conflict("Article is not awaiting review".into()));
    }
    let review_id = sqlx::query_scalar::<_, Uuid>("SELECT id FROM article_reviews WHERE article_id = ? AND organization_id = ? AND status = 'PENDING' ORDER BY created_at DESC LIMIT 1 FOR UPDATE")
        .bind(id).bind(identity.organization_id).fetch_optional(&mut *tx).await?.ok_or(AppError::NotFound("Pending review not found"))?;
    sqlx::query("UPDATE article_reviews SET status = ?, reviewer_id = ?, decision_comment = ?, decided_at = CURRENT_TIMESTAMP(6) WHERE id = ?")
        .bind(review_status).bind(identity.user_id).bind(comment.clone()).bind(review_id).execute(&mut *tx).await?;
    sqlx::query(
        "UPDATE articles SET status = ?, last_reviewed_at = CURRENT_TIMESTAMP(6) WHERE id = ?",
    )
    .bind(article_status)
    .bind(id)
    .execute(&mut *tx)
    .await?;
    sqlx::query("INSERT INTO notifications (id, organization_id, user_id, event_type, title, body, resource_type, resource_id) VALUES (?, ?, ?, ?, ?, ?, 'article', ?)")
        .bind(Uuid::new_v4()).bind(identity.organization_id).bind(article.author_id).bind(action).bind(format!("Review decision: {}", article.title)).bind(comment).bind(id).execute(&mut *tx).await?;
    audit(&mut tx, identity, action, "article", id).await?;
    tx.commit().await?;
    Ok(())
}

async fn snapshot(
    tx: &mut Transaction<'_, MySql>,
    organization_id: Uuid,
    article_id: Uuid,
    author_id: Uuid,
    summary: Option<String>,
) -> Result<Uuid, AppError> {
    let number = sqlx::query_scalar::<_, u32>(
        "SELECT COALESCE(MAX(version_number), 0) + 1 FROM article_versions WHERE article_id = ?",
    )
    .bind(article_id)
    .fetch_one(&mut **tx)
    .await?;
    let article = sqlx::query_as::<_, (String, String, String, Option<String>, String)>("SELECT title, draft_content, visibility, short_description, language FROM articles WHERE id = ? AND organization_id = ? FOR UPDATE")
        .bind(article_id).bind(organization_id).fetch_one(&mut **tx).await?;
    let metadata = serde_json::json!({"visibility": article.2, "short_description": article.3, "language": article.4});
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO article_versions (id, organization_id, article_id, version_number, title, content, metadata, author_id, change_summary) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(id).bind(organization_id).bind(article_id).bind(number).bind(article.0).bind(article.1).bind(metadata).bind(author_id).bind(summary).execute(&mut **tx).await?;
    Ok(id)
}

async fn lock_article(
    tx: &mut Transaction<'_, MySql>,
    organization_id: Uuid,
    id: Uuid,
) -> Result<Article, AppError> {
    sqlx::query_as::<_, Article>(&format!(
        "{} FOR UPDATE",
        article_select("id = ? AND organization_id = ? AND deleted_at IS NULL")
    ))
    .bind(id)
    .bind(organization_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(AppError::NotFound("Article not found"))
}

pub(crate) async fn get_scoped(
    state: &AppState,
    organization_id: Uuid,
    id: Uuid,
) -> Result<Article, AppError> {
    sqlx::query_as::<_, Article>(&article_select(
        "id = ? AND organization_id = ? AND deleted_at IS NULL",
    ))
    .bind(id)
    .bind(organization_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound("Article not found"))
}

fn article_select(predicate: &str) -> String {
    format!(
        "SELECT id, knowledge_base_id, category_id, author_id, current_version_id, title, slug, short_description, draft_content, status, visibility, language, featured, pinned, seo_title, seo_description, canonical_url, published_at, scheduled_publish_at, lock_version, created_at, updated_at FROM articles WHERE {predicate}"
    )
}

async fn ensure_category(
    tx: &mut Transaction<'_, MySql>,
    organization_id: Uuid,
    kb_id: Uuid,
    category_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query_scalar::<_, i8>("SELECT 1 FROM categories WHERE id = ? AND knowledge_base_id = ? AND organization_id = ? AND deleted_at IS NULL")
        .bind(category_id).bind(kb_id).bind(organization_id).fetch_optional(&mut **tx).await?.map(|_| ()).ok_or(AppError::NotFound("Category not found"))
}

async fn audit(
    tx: &mut Transaction<'_, MySql>,
    identity: &Identity,
    action: &str,
    resource_type: &str,
    resource_id: Uuid,
) -> Result<(), AppError> {
    sqlx::query("INSERT INTO audit_logs (id, organization_id, user_id, action, resource_type, resource_id) VALUES (?, ?, ?, ?, ?, ?)")
        .bind(Uuid::new_v4()).bind(identity.organization_id).bind(identity.user_id).bind(action).bind(resource_type).bind(resource_id).execute(&mut **tx).await?;
    Ok(())
}

fn ensure_editable(status: &str) -> Result<(), AppError> {
    matches!(status, "DRAFT").then_some(()).ok_or_else(|| {
        AppError::Conflict(
            "Only draft articles can be edited. Restore or create a draft first.".into(),
        )
    })
}

fn transition_allowed(from: &str, to: &str) -> Result<(), AppError> {
    let allowed = matches!(
        (from, to),
        ("APPROVED", "PUBLISHED") | ("PUBLISHED", "ARCHIVED") | ("ARCHIVED", "DRAFT")
    );
    allowed
        .then_some(())
        .ok_or_else(|| AppError::Conflict(format!("Invalid article transition: {from} → {to}")))
}

fn validate_slug(slug: &str) -> Result<(), AppError> {
    let valid = !slug.is_empty()
        && slug.len() <= 260
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--");
    valid
        .then_some(())
        .ok_or_else(|| AppError::Validation("Invalid article slug".into()))
}

fn map_unique(error: sqlx::Error) -> AppError {
    if matches!(&error, sqlx::Error::Database(database) if database.is_unique_violation()) {
        AppError::Conflict("This knowledge base already uses that article slug".into())
    } else {
        AppError::Database(error)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn published_content_cannot_be_edited_in_place() {
        assert!(ensure_editable("PUBLISHED").is_err());
    }
    #[test]
    fn lifecycle_rejects_publish_from_draft() {
        assert!(transition_allowed("APPROVED", "PUBLISHED").is_ok());
        assert!(transition_allowed("DRAFT", "PUBLISHED").is_err());
        assert!(transition_allowed("ARCHIVED", "DRAFT").is_ok());
    }
}
