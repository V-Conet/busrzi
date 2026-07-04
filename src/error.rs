use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("redis error: {0}")]
    Redis(#[from] redis::RedisError),

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            AppError::InvalidUrl(msg) => (StatusCode::BAD_REQUEST, "invalid_url", msg.clone()),
            AppError::Redis(_) => (StatusCode::INTERNAL_SERVER_ERROR, "redis_error", self.to_string()),
            AppError::Io(_) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", self.to_string()),
        };

        let body = Json(json!({
            "ok": false,
            "error": {
                "code": code,
                "message": message,
            }
        }));

        (status, body).into_response()
    }
}
