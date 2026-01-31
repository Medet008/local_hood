use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Не авторизован")]
    Unauthorized,

    #[error("Доступ запрещён")]
    Forbidden,

    #[error("Не найдено: {0}")]
    NotFound(String),

    #[error("Неверный запрос: {0}")]
    BadRequest(String),

    #[error("Конфликт: {0}")]
    Conflict(String),

    #[error("Ошибка валидации: {0}")]
    Validation(String),

    #[error("Слишком много запросов")]
    TooManyRequests,

    #[error("Ошибка базы данных: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Ошибка JWT: {0}")]
    Jwt(#[from] jsonwebtoken::errors::Error),

    #[error("Внутренняя ошибка: {0}")]
    Internal(String),

    #[error("Ошибка SMS: {0}")]
    Sms(String),

    #[error("Ошибка файла: {0}")]
    File(String),

    #[error("Код подтверждения истёк")]
    CodeExpired,

    #[error("Неверный код подтверждения")]
    InvalidCode,

    #[error("Превышено количество попыток")]
    TooManyAttempts,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_code, message) = match &self {
            AppError::Unauthorized => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", self.to_string()),
            AppError::Forbidden => (StatusCode::FORBIDDEN, "FORBIDDEN", self.to_string()),
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            AppError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            AppError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                msg.clone(),
            ),
            AppError::TooManyRequests => (
                StatusCode::TOO_MANY_REQUESTS,
                "TOO_MANY_REQUESTS",
                self.to_string(),
            ),
            AppError::Database(e) => {
                tracing::error!("Database error: {:?}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "DATABASE_ERROR",
                    "Ошибка базы данных".to_string(),
                )
            }
            AppError::Jwt(_) => (
                StatusCode::UNAUTHORIZED,
                "INVALID_TOKEN",
                "Неверный токен".to_string(),
            ),
            AppError::Internal(msg) => {
                tracing::error!("Internal error: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "Внутренняя ошибка".to_string(),
                )
            }
            AppError::Sms(msg) => (StatusCode::SERVICE_UNAVAILABLE, "SMS_ERROR", msg.clone()),
            AppError::File(msg) => (StatusCode::BAD_REQUEST, "FILE_ERROR", msg.clone()),
            AppError::CodeExpired => (StatusCode::BAD_REQUEST, "CODE_EXPIRED", self.to_string()),
            AppError::InvalidCode => (StatusCode::BAD_REQUEST, "INVALID_CODE", self.to_string()),
            AppError::TooManyAttempts => (
                StatusCode::TOO_MANY_REQUESTS,
                "TOO_MANY_ATTEMPTS",
                self.to_string(),
            ),
        };

        let body = Json(json!({
            "success": false,
            "error": {
                "code": error_code,
                "message": message
            }
        }));

        (status, body).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
