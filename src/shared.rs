use serde::Serialize;

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiResponse<T: Serialize> {
    pub data: T,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<serde_json::Value>,
    pub error: Option<ApiErrorBody>,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ApiErrorBody {
    pub code: String,
    pub message: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn data(data: T) -> Self {
        Self {
            data,
            meta: None,
            error: None,
        }
    }
}
