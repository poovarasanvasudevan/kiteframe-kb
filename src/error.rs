use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::Value;
use thiserror::Error;
use tracing::error;

use crate::shared::{ApiErrorBody, ApiResponse};

#[derive(Debug, Error)]
pub enum AppError {
    #[error("authentication required")]
    Unauthorized,
    #[error("you do not have permission to perform this action")]
    Forbidden,
    #[error("{0}")]
    NotFound(&'static str),
    #[error("{0}")]
    Validation(String),
    #[error("{0}")]
    Conflict(String),
    #[error(transparent)]
    Database(#[from] sqlx::Error),
    #[error(transparent)]
    Internal(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            Self::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", self.to_string()),
            Self::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", self.to_string()),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "NOT_FOUND", self.to_string()),
            Self::Validation(_) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                self.to_string(),
            ),
            Self::Conflict(_) => (StatusCode::CONFLICT, "CONFLICT", self.to_string()),
            Self::Database(error) => {
                error!(?error, "database request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "The request could not be completed".into(),
                )
            }
            Self::Internal(error) => {
                error!(?error, "internal request failed");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "The request could not be completed".into(),
                )
            }
        };
        (
            status,
            Json(ApiResponse {
                data: Value::Null,
                meta: None,
                error: Some(ApiErrorBody {
                    code: code.into(),
                    message,
                }),
            }),
        )
            .into_response()
    }
}
