use axum::{
    body::Body,
    extract::State,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use sqlx::PgPool;
use uuid::Uuid;

use crate::config::Config;
use crate::models::UserRole;
use crate::services::AuthService;

#[derive(Clone, Debug)]
pub struct AuthUser {
    pub user_id: Uuid,
    pub role: UserRole,
}

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
}

// Вспомогательные функции для проверки ролей
pub fn is_chairman_or_higher(role: &UserRole) -> bool {
    matches!(
        role,
        UserRole::Chairman | UserRole::Admin | UserRole::SuperAdmin
    )
}

pub fn is_admin_or_higher(role: &UserRole) -> bool {
    matches!(role, UserRole::Admin | UserRole::SuperAdmin)
}

pub fn is_owner_or_higher(role: &UserRole) -> bool {
    matches!(
        role,
        UserRole::Owner
            | UserRole::Council
            | UserRole::Chairman
            | UserRole::Admin
            | UserRole::SuperAdmin
    )
}

pub fn is_resident_or_higher(role: &UserRole) -> bool {
    !matches!(role, UserRole::User)
}

fn parse_role(role_str: &str) -> UserRole {
    match role_str {
        "user" => UserRole::User,
        "resident" => UserRole::Resident,
        "owner" => UserRole::Owner,
        "council" => UserRole::Council,
        "chairman" => UserRole::Chairman,
        "moderator" => UserRole::Moderator,
        "admin" => UserRole::Admin,
        "superadmin" | "super_admin" => UserRole::SuperAdmin,
        _ => UserRole::User,
    }
}

// Middleware для добавления AppState в extensions
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    request.extensions_mut().insert(state);
    next.run(request).await
}

// Экстрактор для авторизованного пользователя
#[axum::async_trait]
impl<S> axum::extract::FromRequestParts<S> for AuthUser
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> Result<Self, Self::Rejection> {
        // Получаем AppState из extensions
        let app_state = parts.extensions.get::<AppState>().cloned().ok_or_else(|| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error": "Internal server error"})),
            )
                .into_response()
        })?;

        // Извлекаем токен из заголовка
        let auth_header = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| {
                (
                    StatusCode::UNAUTHORIZED,
                    Json(json!({"error": "Missing authorization header"})),
                )
                    .into_response()
            })?;

        let token = auth_header.strip_prefix("Bearer ").ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid authorization header format"})),
            )
                .into_response()
        })?;

        // Проверяем токен
        let auth_service = AuthService::new(app_state.config);
        let claims = auth_service.verify_token(token).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid or expired token"})),
            )
                .into_response()
        })?;

        // Проверяем тип токена
        if claims.token_type != "access" {
            return Err((
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid token type"})),
            )
                .into_response());
        }

        // Парсим user_id
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| {
            (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "Invalid user ID in token"})),
            )
                .into_response()
        })?;

        // Парсим роль
        let role = parse_role(&claims.role);

        Ok(AuthUser { user_id, role })
    }
}
