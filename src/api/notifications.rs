use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{AppState, AuthUser};
use crate::models::{
    Notification, NotificationResponse, NotificationsQuery, RegisterPushTokenRequest,
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct NotificationSuccessResponse {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct MarkAllReadResponse {
    pub success: bool,
    pub count: i64,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct UnreadCountResponse {
    pub count: i64,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_notifications))
        .route("/:id/read", put(mark_as_read))
        .route("/read-all", post(mark_all_as_read))
        .route("/push-token", post(register_push_token))
        .route("/unread-count", get(get_unread_count))
}

/// Получить список уведомлений пользователя
#[utoipa::path(
    get,
    path = "/api/notifications",
    tag = "Уведомления",
    security(("bearer_auth" = [])),
    params(
        ("limit" = Option<i64>, Query, description = "Лимит записей"),
        ("page" = Option<i64>, Query, description = "Номер страницы"),
        ("unread_only" = Option<bool>, Query, description = "Только непрочитанные")
    ),
    responses(
        (status = 200, description = "Список уведомлений", body = Vec<NotificationResponse>),
        (status = 401, description = "Не авторизован")
    )
)]
async fn list_notifications(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<NotificationsQuery>,
) -> AppResult<Json<Vec<NotificationResponse>>> {
    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.page.unwrap_or(0) * limit;

    let notifications = sqlx::query_as::<_, Notification>(
        r#"
        SELECT * FROM notifications
        WHERE user_id = $1
          AND ($2::boolean IS NULL OR ($2 = true AND is_read = false))
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(auth_user.user_id)
    .bind(query.unread_only)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let response: Vec<NotificationResponse> = notifications
        .into_iter()
        .map(NotificationResponse::from)
        .collect();

    Ok(Json(response))
}

/// Отметить уведомление как прочитанное
#[utoipa::path(
    put,
    path = "/api/notifications/{id}/read",
    tag = "Уведомления",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID уведомления")
    ),
    responses(
        (status = 200, description = "Уведомление отмечено", body = NotificationSuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Уведомление не найдено")
    )
)]
async fn mark_as_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = true, read_at = NOW() WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth_user.user_id)
    .execute(&state.pool)
    .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Уведомление не найдено".to_string()));
    }

    Ok(Json(json!({"success": true})))
}

/// Отметить все уведомления как прочитанные
#[utoipa::path(
    post,
    path = "/api/notifications/read-all",
    tag = "Уведомления",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Все уведомления отмечены", body = MarkAllReadResponse),
        (status = 401, description = "Не авторизован")
    )
)]
async fn mark_all_as_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Value>> {
    let result = sqlx::query(
        "UPDATE notifications SET is_read = true, read_at = NOW() WHERE user_id = $1 AND is_read = false"
    )
    .bind(auth_user.user_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "count": result.rows_affected()
    })))
}

/// Зарегистрировать push-токен устройства
#[utoipa::path(
    post,
    path = "/api/notifications/push-token",
    tag = "Уведомления",
    security(("bearer_auth" = [])),
    request_body = RegisterPushTokenRequest,
    responses(
        (status = 200, description = "Токен зарегистрирован", body = NotificationSuccessResponse),
        (status = 401, description = "Не авторизован")
    )
)]
async fn register_push_token(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<RegisterPushTokenRequest>,
) -> AppResult<Json<Value>> {
    sqlx::query(
        r#"
        INSERT INTO push_tokens (user_id, token, platform, device_id)
        VALUES ($1, $2, $3, $4)
        ON CONFLICT (user_id, token) DO UPDATE SET
            platform = EXCLUDED.platform,
            device_id = EXCLUDED.device_id,
            is_active = true,
            updated_at = NOW()
        "#,
    )
    .bind(auth_user.user_id)
    .bind(&payload.token)
    .bind(&payload.platform)
    .bind(&payload.device_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({"success": true})))
}

/// Получить количество непрочитанных уведомлений
#[utoipa::path(
    get,
    path = "/api/notifications/unread-count",
    tag = "Уведомления",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Количество непрочитанных", body = UnreadCountResponse),
        (status = 401, description = "Не авторизован")
    )
)]
async fn get_unread_count(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Value>> {
    let count: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM notifications WHERE user_id = $1 AND is_read = false")
            .bind(auth_user.user_id)
            .fetch_one(&state.pool)
            .await?;

    Ok(Json(json!({"count": count.0})))
}
