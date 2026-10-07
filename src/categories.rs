use axum::{
    Json,
    extract::{Path, State},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, MySql, Transaction};
use uuid::Uuid;
use validator::Validate;

use crate::{
    app::AppState, auth::Identity, error::AppError, knowledge_bases::Visibility,
    shared::ApiResponse,
};

#[derive(Debug, Clone, Serialize, FromRow, utoipa::ToSchema)]
pub struct Category {
    pub id: Uuid,
    pub knowledge_base_id: Uuid,
    pub parent_id: Option<Uuid>,
    pub name: String,
    pub slug: String,
    pub description: Option<String>,
    pub visibility: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct CreateCategory {
    pub knowledge_base_id: Uuid,
    pub parent_id: Option<Uuid>,
    #[validate(length(min = 2, max = 120))]
    pub name: String,
    #[validate(length(min = 2, max = 140))]
    pub slug: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub visibility: Visibility,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct MoveCategory {
    pub parent_id: Option<Uuid>,
    pub position: u32,
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct UpdateCategory {
    #[validate(length(min = 2, max = 120))]
    pub name: String,
    #[validate(length(max = 1000))]
    pub description: Option<String>,
    pub visibility: Visibility,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ReorderCategories {
    pub category_ids: Vec<Uuid>,
}

#[utoipa::path(get, path = "/api/v1/categories/by-knowledge-base/{knowledge_base_id}", params(("knowledge_base_id" = Uuid, Path)), security(("bearer_auth" = [])), responses((status = 200, body = [Category])))]
pub async fn list(
    State(state): State<AppState>,
    identity: Identity,
    Path(knowledge_base_id): Path<Uuid>,
) -> Result<Json<ApiResponse<Vec<Category>>>, AppError> {
    identity.require("kb.read")?;
    ensure_kb(&state, identity.organization_id, knowledge_base_id).await?;
    let categories = sqlx::query_as::<_, Category>(
        "SELECT id, knowledge_base_id, parent_id, name, slug, description, visibility, sort_order, created_at, updated_at FROM categories WHERE organization_id = ? AND knowledge_base_id = ? AND deleted_at IS NULL ORDER BY parent_id, sort_order, name",
    )
    .bind(identity.organization_id)
    .bind(knowledge_base_id)
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(ApiResponse::data(categories)))
}

#[utoipa::path(post, path = "/api/v1/categories", request_body = CreateCategory, security(("bearer_auth" = [])), responses((status = 200, body = Category)))]
pub async fn create(
    State(state): State<AppState>,
    identity: Identity,
    Json(input): Json<CreateCategory>,
) -> Result<Json<ApiResponse<Category>>, AppError> {
    identity.require("category.manage")?;
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    validate_slug(&input.slug)?;
    ensure_kb(&state, identity.organization_id, input.knowledge_base_id).await?;
    if let Some(parent_id) = input.parent_id {
        ensure_category_scope(
            &state,
            identity.organization_id,
            input.knowledge_base_id,
            parent_id,
        )
        .await?;
    }
    let sort_order = sqlx::query_scalar::<_, i64>(
        "SELECT COALESCE(MAX(sort_order), -1) + 1 FROM categories WHERE organization_id = ? AND knowledge_base_id = ? AND parent_id <=> ? AND deleted_at IS NULL",
    )
    .bind(identity.organization_id)
    .bind(input.knowledge_base_id)
    .bind(input.parent_id)
    .fetch_one(&state.pool)
    .await?;
    let id = Uuid::new_v4();
    let result = sqlx::query("INSERT INTO categories (id, organization_id, knowledge_base_id, parent_id, name, slug, description, visibility, sort_order, created_by) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)")
        .bind(id).bind(identity.organization_id).bind(input.knowledge_base_id).bind(input.parent_id)
        .bind(input.name.trim()).bind(input.slug.trim().to_lowercase()).bind(input.description)
        .bind(input.visibility.as_str()).bind(sort_order).bind(identity.user_id).execute(&state.pool).await;
    if let Err(sqlx::Error::Database(error)) = &result {
        if error.is_unique_violation() {
            return Err(AppError::Conflict(
                "This knowledge base already uses that category slug".into(),
            ));
        }
    }
    result?;
    Ok(Json(ApiResponse::data(
        get_category(&state, identity.organization_id, id).await?,
    )))
}

#[utoipa::path(post, path = "/api/v1/categories/{id}/move", params(("id" = Uuid, Path)), request_body = MoveCategory, security(("bearer_auth" = [])), responses((status = 200, body = Category)))]
pub async fn move_category(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
    Json(input): Json<MoveCategory>,
) -> Result<Json<ApiResponse<Category>>, AppError> {
    identity.require("category.manage")?;
    validate_move(id, input.parent_id)?;
    let mut transaction = state.pool.begin().await?;
    let category = lock_category(&mut transaction, identity.organization_id, id).await?;
    if let Some(parent_id) = input.parent_id {
        let parent = lock_category(&mut transaction, identity.organization_id, parent_id).await?;
        if parent.knowledge_base_id != category.knowledge_base_id {
            return Err(AppError::Validation(
                "Categories cannot move between knowledge bases".into(),
            ));
        }
        let is_descendant = sqlx::query_scalar::<_, i8>(
            "WITH RECURSIVE descendants AS (SELECT id FROM categories WHERE parent_id = ? AND organization_id = ? AND deleted_at IS NULL UNION ALL SELECT c.id FROM categories c JOIN descendants d ON c.parent_id = d.id WHERE c.organization_id = ? AND c.deleted_at IS NULL) SELECT EXISTS(SELECT 1 FROM descendants WHERE id = ?)",
        )
        .bind(id).bind(identity.organization_id).bind(identity.organization_id).bind(parent_id)
        .fetch_one(&mut *transaction).await? != 0;
        if is_descendant {
            return Err(AppError::Validation(
                "A category cannot move inside one of its descendants".into(),
            ));
        }
    }
    sqlx::query("UPDATE categories SET sort_order = sort_order + 1 WHERE organization_id = ? AND knowledge_base_id = ? AND parent_id <=> ? AND sort_order >= ? AND deleted_at IS NULL")
        .bind(identity.organization_id).bind(category.knowledge_base_id).bind(input.parent_id).bind(input.position)
        .execute(&mut *transaction).await?;
    sqlx::query(
        "UPDATE categories SET parent_id = ?, sort_order = ? WHERE id = ? AND organization_id = ?",
    )
    .bind(input.parent_id)
    .bind(input.position)
    .bind(id)
    .bind(identity.organization_id)
    .execute(&mut *transaction)
    .await?;
    transaction.commit().await?;
    Ok(Json(ApiResponse::data(
        get_category(&state, identity.organization_id, id).await?,
    )))
}

pub async fn update(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
    Json(input): Json<UpdateCategory>,
) -> Result<Json<ApiResponse<Category>>, AppError> {
    identity.require("category.manage")?;
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let changed = sqlx::query("UPDATE categories SET name = ?, description = ?, visibility = ? WHERE id = ? AND organization_id = ? AND deleted_at IS NULL")
        .bind(input.name.trim()).bind(input.description).bind(input.visibility.as_str()).bind(id).bind(identity.organization_id).execute(&state.pool).await?.rows_affected();
    if changed == 0 {
        return Err(AppError::NotFound("Category not found"));
    }
    Ok(Json(ApiResponse::data(
        get_category(&state, identity.organization_id, id).await?,
    )))
}

pub async fn delete(
    State(state): State<AppState>,
    identity: Identity,
    Path(id): Path<Uuid>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    identity.require("category.manage")?;
    let mut tx = state.pool.begin().await?;
    lock_category(&mut tx, identity.organization_id, id).await?;
    let child_count = sqlx::query_scalar::<_, i64>("SELECT (SELECT COUNT(*) FROM categories WHERE parent_id = ? AND organization_id = ? AND deleted_at IS NULL) + (SELECT COUNT(*) FROM articles WHERE category_id = ? AND organization_id = ? AND deleted_at IS NULL)")
        .bind(id).bind(identity.organization_id).bind(id).bind(identity.organization_id).fetch_one(&mut *tx).await?;
    if child_count > 0 {
        return Err(AppError::Conflict(
            "Move or delete child categories and articles before deleting this category".into(),
        ));
    }
    sqlx::query("UPDATE categories SET deleted_at = CURRENT_TIMESTAMP(6) WHERE id = ? AND organization_id = ?").bind(id).bind(identity.organization_id).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(Json(ApiResponse::data(
        serde_json::json!({"deleted": true}),
    )))
}

#[utoipa::path(post, path = "/api/v1/categories/reorder", request_body = ReorderCategories, security(("bearer_auth" = [])), responses((status = 200, body = [Category])))]
pub async fn reorder(
    State(state): State<AppState>,
    identity: Identity,
    Json(input): Json<ReorderCategories>,
) -> Result<Json<ApiResponse<Vec<Category>>>, AppError> {
    identity.require("category.manage")?;
    if input.category_ids.is_empty() || input.category_ids.len() > 500 {
        return Err(AppError::Validation(
            "Reorder between 1 and 500 categories at a time".into(),
        ));
    }
    let mut transaction = state.pool.begin().await?;
    let mut scope: Option<(Uuid, Option<Uuid>)> = None;
    for (position, id) in input.category_ids.iter().enumerate() {
        let category = lock_category(&mut transaction, identity.organization_id, *id).await?;
        let candidate = (category.knowledge_base_id, category.parent_id);
        if scope.is_some_and(|expected| expected != candidate) {
            return Err(AppError::Validation(
                "Only sibling categories can be reordered together".into(),
            ));
        }
        scope = Some(candidate);
        sqlx::query("UPDATE categories SET sort_order = ? WHERE id = ? AND organization_id = ?")
            .bind(position as u32)
            .bind(id)
            .bind(identity.organization_id)
            .execute(&mut *transaction)
            .await?;
    }
    transaction.commit().await?;
    let records =
        futures_for_categories(&state, identity.organization_id, &input.category_ids).await?;
    Ok(Json(ApiResponse::data(records)))
}

async fn lock_category(
    transaction: &mut Transaction<'_, MySql>,
    organization_id: Uuid,
    id: Uuid,
) -> Result<Category, AppError> {
    sqlx::query_as::<_, Category>("SELECT id, knowledge_base_id, parent_id, name, slug, description, visibility, sort_order, created_at, updated_at FROM categories WHERE id = ? AND organization_id = ? AND deleted_at IS NULL FOR UPDATE")
        .bind(id).bind(organization_id).fetch_optional(&mut **transaction).await?.ok_or(AppError::NotFound("Category not found"))
}

async fn get_category(
    state: &AppState,
    organization_id: Uuid,
    id: Uuid,
) -> Result<Category, AppError> {
    sqlx::query_as::<_, Category>("SELECT id, knowledge_base_id, parent_id, name, slug, description, visibility, sort_order, created_at, updated_at FROM categories WHERE id = ? AND organization_id = ? AND deleted_at IS NULL")
        .bind(id).bind(organization_id).fetch_optional(&state.pool).await?.ok_or(AppError::NotFound("Category not found"))
}

async fn futures_for_categories(
    state: &AppState,
    organization_id: Uuid,
    ids: &[Uuid],
) -> Result<Vec<Category>, AppError> {
    let mut records = Vec::with_capacity(ids.len());
    for id in ids {
        records.push(get_category(state, organization_id, *id).await?);
    }
    Ok(records)
}

async fn ensure_kb(state: &AppState, organization_id: Uuid, id: Uuid) -> Result<(), AppError> {
    sqlx::query_scalar::<_, i8>(
        "SELECT 1 FROM knowledge_bases WHERE id = ? AND organization_id = ? AND deleted_at IS NULL",
    )
    .bind(id)
    .bind(organization_id)
    .fetch_optional(&state.pool)
    .await?
    .map(|_| ())
    .ok_or(AppError::NotFound("Knowledge base not found"))
}

async fn ensure_category_scope(
    state: &AppState,
    organization_id: Uuid,
    knowledge_base_id: Uuid,
    id: Uuid,
) -> Result<(), AppError> {
    sqlx::query_scalar::<_, i8>("SELECT 1 FROM categories WHERE id = ? AND organization_id = ? AND knowledge_base_id = ? AND deleted_at IS NULL")
        .bind(id).bind(organization_id).bind(knowledge_base_id).fetch_optional(&state.pool).await?.map(|_| ()).ok_or(AppError::NotFound("Parent category not found"))
}

fn validate_slug(slug: &str) -> Result<(), AppError> {
    let valid = !slug.is_empty()
        && slug.len() <= 140
        && slug
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !slug.starts_with('-')
        && !slug.ends_with('-')
        && !slug.contains("--");
    valid.then_some(()).ok_or_else(|| {
        AppError::Validation(
            "Slug must contain lowercase letters, numbers, and single hyphens".into(),
        )
    })
}

fn validate_move(id: Uuid, parent_id: Option<Uuid>) -> Result<(), AppError> {
    if parent_id == Some(id) {
        return Err(AppError::Validation(
            "A category cannot be its own parent".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn self_parenting_is_rejected() {
        let id = Uuid::new_v4();
        assert!(validate_move(id, Some(id)).is_err());
        assert!(validate_move(id, None).is_ok());
    }
    #[test]
    fn category_slug_rejects_repeated_hyphens() {
        assert!(validate_slug("getting-started").is_ok());
        assert!(validate_slug("getting--started").is_err());
    }
}
