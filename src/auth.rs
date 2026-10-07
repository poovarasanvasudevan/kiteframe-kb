use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordVerifier};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::request::Parts,
};
use chrono::{Duration, Utc};
use jsonwebtoken::{DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, MySqlPool};
use uuid::Uuid;
use validator::Validate;

use crate::{app::AppState, error::AppError, shared::ApiResponse};

#[derive(Clone)]
pub struct JwtService {
    encoding: EncodingKey,
    decoding: DecodingKey,
    expiration_seconds: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: Uuid,
    organization_id: Uuid,
    permissions: Vec<String>,
    exp: usize,
    iat: usize,
}

#[derive(Debug, Clone)]
pub struct Identity {
    pub user_id: Uuid,
    pub organization_id: Uuid,
    pub permissions: Arc<[String]>,
}

impl Identity {
    pub fn require(&self, permission: &str) -> Result<(), AppError> {
        self.permissions
            .iter()
            .any(|candidate| candidate == permission)
            .then_some(())
            .ok_or(AppError::Forbidden)
    }
}

impl JwtService {
    pub fn new(secret: &[u8], expiration_seconds: i64) -> Self {
        Self {
            encoding: EncodingKey::from_secret(secret),
            decoding: DecodingKey::from_secret(secret),
            expiration_seconds,
        }
    }

    fn issue(&self, user: &UserWithPassword, permissions: Vec<String>) -> Result<String, AppError> {
        let now = Utc::now();
        let claims = Claims {
            sub: user.id,
            organization_id: user.organization_id,
            permissions,
            iat: now.timestamp() as usize,
            exp: (now + Duration::seconds(self.expiration_seconds)).timestamp() as usize,
        };
        jsonwebtoken::encode(&Header::default(), &claims, &self.encoding)
            .map_err(|error| AppError::Internal(error.into()))
    }

    fn decode(&self, token: &str) -> Result<Identity, AppError> {
        let token = jsonwebtoken::decode::<Claims>(token, &self.decoding, &Validation::default())
            .map_err(|_| AppError::Unauthorized)?;
        Ok(Identity {
            user_id: token.claims.sub,
            organization_id: token.claims.organization_id,
            permissions: token.claims.permissions.into(),
        })
    }
}

impl FromRequestParts<AppState> for Identity {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let header = parts
            .headers
            .get("authorization")
            .and_then(|value| value.to_str().ok())
            .ok_or(AppError::Unauthorized)?;
        let token = header
            .strip_prefix("Bearer ")
            .ok_or(AppError::Unauthorized)?;
        state.jwt.decode(token)
    }
}

#[derive(Debug, Deserialize, Validate, utoipa::ToSchema)]
pub struct LoginRequest {
    #[validate(email)]
    pub email: String,
    #[validate(length(min = 8, max = 128))]
    pub password: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub expires_in: i64,
    pub user: AuthenticatedUser,
}

#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AuthenticatedUser {
    pub id: Uuid,
    pub organization_id: Uuid,
    pub name: String,
    pub email: String,
    pub permissions: Vec<String>,
}

#[derive(Debug, FromRow)]
struct UserWithPassword {
    id: Uuid,
    organization_id: Uuid,
    name: String,
    email: String,
    password_hash: String,
}

#[utoipa::path(post, path = "/api/v1/auth/login", request_body = LoginRequest,
    responses((status = 200, body = LoginResponse), (status = 401, description = "Invalid credentials")))]
pub async fn login(
    State(state): State<AppState>,
    Json(input): Json<LoginRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    input
        .validate()
        .map_err(|error| AppError::Validation(error.to_string()))?;
    let user = sqlx::query_as::<_, UserWithPassword>(
        "SELECT id, organization_id, name, email, password_hash FROM users WHERE email = ? AND status = 'ACTIVE' AND deleted_at IS NULL"
    ).bind(input.email.trim().to_lowercase()).fetch_optional(&state.pool).await?
        .ok_or(AppError::Unauthorized)?;
    let password_hash =
        PasswordHash::new(&user.password_hash).map_err(|_| AppError::Unauthorized)?;
    Argon2::default()
        .verify_password(input.password.as_bytes(), &password_hash)
        .map_err(|_| AppError::Unauthorized)?;

    let permissions = load_permissions(&state.pool, user.id).await?;
    let access_token = state.jwt.issue(&user, permissions.clone())?;
    let refresh_token = format!("{}.{}", Uuid::new_v4(), Uuid::new_v4());
    let refresh_hash = Sha256::digest(refresh_token.as_bytes()).to_vec();
    sqlx::query("INSERT INTO refresh_tokens (id, user_id, token_hash, family_id, expires_at) VALUES (?, ?, ?, ?, DATE_ADD(CURRENT_TIMESTAMP(6), INTERVAL ? SECOND))")
        .bind(Uuid::new_v4()).bind(user.id).bind(refresh_hash).bind(Uuid::new_v4()).bind(state.config.jwt_refresh_expiration_seconds).execute(&state.pool).await?;
    Ok(Json(ApiResponse::data(LoginResponse {
        access_token,
        refresh_token,
        token_type: "Bearer",
        expires_in: state.config.jwt_access_expiration_seconds,
        user: AuthenticatedUser {
            id: user.id,
            organization_id: user.organization_id,
            name: user.name,
            email: user.email,
            permissions,
        },
    })))
}

pub async fn refresh(
    State(state): State<AppState>,
    Json(input): Json<RefreshRequest>,
) -> Result<Json<ApiResponse<LoginResponse>>, AppError> {
    let hash = Sha256::digest(input.refresh_token.as_bytes()).to_vec();
    let mut tx = state.pool.begin().await?;
    let record = sqlx::query_as::<_, (Uuid, Uuid, Uuid)>("SELECT rt.id, rt.user_id, rt.family_id FROM refresh_tokens rt WHERE rt.token_hash = ? AND rt.revoked_at IS NULL AND rt.expires_at > CURRENT_TIMESTAMP(6) FOR UPDATE")
        .bind(hash).fetch_optional(&mut *tx).await?.ok_or(AppError::Unauthorized)?;
    let user = sqlx::query_as::<_, UserWithPassword>("SELECT id, organization_id, name, email, password_hash FROM users WHERE id = ? AND status = 'ACTIVE' AND deleted_at IS NULL")
        .bind(record.1).fetch_optional(&mut *tx).await?.ok_or(AppError::Unauthorized)?;
    let new_token = format!("{}.{}", Uuid::new_v4(), Uuid::new_v4());
    let new_id = Uuid::new_v4();
    sqlx::query("UPDATE refresh_tokens SET revoked_at = CURRENT_TIMESTAMP(6), replaced_by_id = ? WHERE id = ?").bind(new_id).bind(record.0).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO refresh_tokens (id, user_id, token_hash, family_id, expires_at) VALUES (?, ?, ?, ?, DATE_ADD(CURRENT_TIMESTAMP(6), INTERVAL ? SECOND))")
        .bind(new_id).bind(user.id).bind(Sha256::digest(new_token.as_bytes()).to_vec()).bind(record.2).bind(state.config.jwt_refresh_expiration_seconds).execute(&mut *tx).await?;
    tx.commit().await?;
    let permissions = load_permissions(&state.pool, user.id).await?;
    let access_token = state.jwt.issue(&user, permissions.clone())?;
    Ok(Json(ApiResponse::data(LoginResponse {
        access_token,
        refresh_token: new_token,
        token_type: "Bearer",
        expires_in: state.config.jwt_access_expiration_seconds,
        user: AuthenticatedUser {
            id: user.id,
            organization_id: user.organization_id,
            name: user.name,
            email: user.email,
            permissions,
        },
    })))
}

pub async fn logout(
    State(state): State<AppState>,
    Json(input): Json<RefreshRequest>,
) -> Result<Json<ApiResponse<serde_json::Value>>, AppError> {
    let hash = Sha256::digest(input.refresh_token.as_bytes()).to_vec();
    sqlx::query("UPDATE refresh_tokens SET revoked_at = COALESCE(revoked_at, CURRENT_TIMESTAMP(6)) WHERE token_hash = ?").bind(hash).execute(&state.pool).await?;
    Ok(Json(ApiResponse::data(
        serde_json::json!({"logged_out": true}),
    )))
}

async fn load_permissions(pool: &MySqlPool, user_id: Uuid) -> Result<Vec<String>, AppError> {
    Ok(sqlx::query_scalar::<_, String>(
        "SELECT DISTINCT p.name FROM permissions p JOIN role_permissions rp ON rp.permission_id = p.id JOIN user_roles ur ON ur.role_id = rp.role_id WHERE ur.user_id = ?"
    ).bind(user_id).fetch_all(pool).await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_round_trip_preserves_tenant_and_permissions() {
        let service = JwtService::new(b"a-test-secret-that-is-at-least-32-bytes", 60);
        let user = UserWithPassword {
            id: Uuid::new_v4(),
            organization_id: Uuid::new_v4(),
            name: "A".into(),
            email: "a@example.com".into(),
            password_hash: String::new(),
        };
        let token = service.issue(&user, vec!["kb.read".into()]).unwrap();
        let identity = service.decode(&token).unwrap();
        assert_eq!(identity.organization_id, user.organization_id);
        assert!(identity.require("kb.read").is_ok());
        assert!(identity.require("kb.delete").is_err());
    }
}
