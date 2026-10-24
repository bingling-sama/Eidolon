use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;

#[derive(Debug)]
pub enum AppError {
    BadRequest(String),
    FetchError(String),
    RenderError(String),
    QueueFull,
    Internal(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::FetchError(msg) => (StatusCode::BAD_GATEWAY, msg),
            AppError::RenderError(msg) => (StatusCode::UNPROCESSABLE_ENTITY, msg),
            AppError::QueueFull => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Server is busy, queue is full".to_string(),
            ),
            AppError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(json!({
            "error": message,
            "status": status.as_u16(),
        }));

        (status, body).into_response()
    }
}
