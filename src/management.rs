use axum::{Json, extract::State};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

use crate::{app::AppState, auth::Identity, error::AppError, shared::ApiResponse};

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct UserSummary {
    pub id: Uuid,
    pub name: String,
    pub email: String,
    pub status: String,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct Team {
    pub id: Uuid,
    pub name: String,
    pub description: Option<String>,
    pub member_count: i64,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateTeam {
    #[validate(length(min = 2, max = 100))]
    pub name: String,
    #[validate(length(max = 500))]
    pub description: Option<String>,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct AuditEntry {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow, utoipa::ToSchema)]
pub struct Notification {
    pub id: Uuid,
    pub event_type: String,
    pub title: String,
    pub body: Option<String>,
    pub resource_type: Option<String>,
    pub resource_id: Option<Uuid>,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct ImportArticle {
    pub knowledge_base_id: Uuid,
    pub category_id: Uuid,
    #[validate(length(min = 2, max = 240))]
    pub title: String,
    #[validate(length(min = 2, max = 260))]
    pub slug: String,
    pub content: String,
    pub visibility: String,
    pub language: Option<String>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct ImportBundle {
    #[validate(nested)]
    pub articles: Vec<ImportArticle>,
}

pub async fn users(
    State(state): State<AppState>,
    identity: Identity,
) -> Result<Json<ApiResponse<Vec<UserSummary>>>, AppError> {
    identity.require("users.manage")?;
    let users = sqlx::query_as::<_, UserSummary>("SELECT id, name, email, status, last_login_at, created_at FROM users WHERE organization_id = ? AND deleted_at IS NULL ORDER BY created_at DESC")
        .bind(identity.organization_id).fetch_all(&state.pool).await?;
    Ok(Json(ApiResponse::data(users)))
}

pub async fn teams(
    State(state): State<AppState>,
    identity: Identity,
) -> Result<Json<ApiResponse<Vec<Team>>>, AppError> {
    identity.require("users.manage")?;
    let teams = sqlx::query_as::<_, Team>("SELECT t.id, t.name, t.description, COUNT(tu.user_id) member_count, t.created_at FROM teams t LEFT JOIN team_users tu ON tu.team_id = t.id WHERE t.organization_id = ? AND t.deleted_at IS NULL GROUP BY t.id, t.name, t.description, t.created_at ORDER BY t.name")
        .bind(identity.organization_id).fetch_all(&state.pool).await?;
    Ok(Json(ApiResponse::data(teams)))
}

pub async fn create_team(
    State(state): State<AppState>,
    identity: Identity,
    Json(input): Json<CreateTeam>,
) -> Result<Json<ApiResponse<Team>>, AppError> {
    identity.require("users.manage")?;
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let id = Uuid::new_v4();
    sqlx::query("INSERT INTO teams (id, organization_id, name, description) VALUES (?, ?, ?, ?)")
        .bind(id)
        .bind(identity.organization_id)
        .bind(input.name.trim())
        .bind(input.description)
        .execute(&state.pool)
        .await
        .map_err(|error| {
            if matches!(&error, sqlx::Error::Database(database) if database.is_unique_violation()) {
                AppError::Conflict("A team already uses that name".into())
            } else {
                AppError::Database(error)
            }
        })?;
    let team = sqlx::query_as::<_, Team>("SELECT t.id, t.name, t.description, 0 member_count, t.created_at FROM teams t WHERE t.id = ? AND t.organization_id = ?")
        .bind(id).bind(identity.organization_id).fetch_one(&state.pool).await?;
    Ok(Json(ApiResponse::data(team)))
}

pub async fn audit_logs(
    State(state): State<AppState>,
    identity: Identity,
) -> Result<Json<ApiResponse<Vec<AuditEntry>>>, AppError> {
    identity.require("audit.read")?;
    let entries = sqlx::query_as::<_, AuditEntry>("SELECT id, user_id, action, resource_type, resource_id, created_at FROM audit_logs WHERE organization_id = ? ORDER BY created_at DESC LIMIT 200")
        .bind(identity.organization_id).fetch_all(&state.pool).await?;
    Ok(Json(ApiResponse::data(entries)))
}

pub async fn notifications(
    State(state): State<AppState>,
    identity: Identity,
) -> Result<Json<ApiResponse<Vec<Notification>>>, AppError> {
    let entries = sqlx::query_as::<_, Notification>("SELECT id, event_type, title, body, resource_type, resource_id, read_at, created_at FROM notifications WHERE organization_id = ? AND user_id = ? ORDER BY read_at IS NULL DESC, created_at DESC LIMIT 100")
        .bind(identity.organization_id).bind(identity.user_id).fetch_all(&state.pool).await?;
    Ok(Json(ApiResponse::data(entries)))
}

pub async fn export_articles(
    State(state): State<AppState>,
    identity: Identity,
) -> Result<Json<ApiResponse<Vec<serde_json::Value>>>, AppError> {
    identity.require("article.read")?;
    let rows = sqlx::query_as::<_, (Uuid, Uuid, String, String, String, String, String)>("SELECT knowledge_base_id, category_id, title, slug, draft_content, visibility, language FROM articles WHERE organization_id = ? AND deleted_at IS NULL ORDER BY created_at")
        .bind(identity.organization_id).fetch_all(&state.pool).await?;
    let data = rows.into_iter().map(|row| serde_json::json!({"knowledge_base_id": row.0, "category_id": row.1, "title": row.2, "slug": row.3, "content": row.4, "visibility": row.5, "language": row.6})).collect();
    Ok(Json(ApiResponse::data(data)))
}

pub async fn import_articles(
    State(state): State<AppState>,
    identity: Identity,
    Json(input): Json<ImportBundle>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    identity.require("article.create")?;
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    if input.articles.is_empty() || input.articles.len() > 500 {
        return Err(AppError::Validation(
            "Import between 1 and 500 articles per request".into(),
        ));
    }
    let mut tx = state.pool.begin().await?;
    let mut imported = 0_u32;
    for item in input.articles {
        let valid_scope = sqlx::query_scalar::<_, i8>("SELECT 1 FROM categories WHERE id = ? AND knowledge_base_id = ? AND organization_id = ? AND deleted_at IS NULL")
            .bind(item.category_id).bind(item.knowledge_base_id).bind(identity.organization_id).fetch_optional(&mut *tx).await?.is_some();
        if !valid_scope {
            return Err(AppError::Validation(format!(
                "Category for '{}' is outside the requested knowledge base",
                item.title
            )));
        }
        let id = Uuid::new_v4();
        let version_id = Uuid::new_v4();
        let visibility = match item.visibility.as_str() {
            "PUBLIC" | "PRIVATE" | "INTERNAL" => item.visibility,
            _ => return Err(AppError::Validation("Invalid article visibility".into())),
        };
        sqlx::query("INSERT INTO articles (id, organization_id, knowledge_base_id, category_id, author_id, title, slug, draft_content, status, visibility, language) VALUES (?, ?, ?, ?, ?, ?, ?, ?, 'DRAFT', ?, ?)")
            .bind(id).bind(identity.organization_id).bind(item.knowledge_base_id).bind(item.category_id).bind(identity.user_id).bind(&item.title).bind(item.slug).bind(ammonia::clean(&item.content)).bind(&visibility).bind(item.language.unwrap_or_else(|| "en".into())).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO article_versions (id, organization_id, article_id, version_number, title, content, metadata, author_id, change_summary) VALUES (?, ?, ?, 1, ?, ?, JSON_OBJECT('visibility', ?), ?, 'Imported')")
            .bind(version_id).bind(identity.organization_id).bind(id).bind(item.title).bind(ammonia::clean(&item.content)).bind(visibility).bind(identity.user_id).execute(&mut *tx).await?;
        sqlx::query("UPDATE articles SET current_version_id = ? WHERE id = ?")
            .bind(version_id)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        imported += 1;
    }
    tx.commit().await?;
    Ok(Json(ApiResponse::data(
        serde_json::json!({"imported": imported}),
    )))
}
