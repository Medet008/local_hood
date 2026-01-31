use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{is_chairman_or_higher, AppState, AuthUser};
use crate::models::{
    Announcement, AnnouncementCategory, AnnouncementPriority, AnnouncementResponse,
    CreateAnnouncementRequest, UpdateAnnouncementRequest,
};

/// Успешный ответ
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_announcements))
        .route("/", post(create_announcement))
        .route("/:id", get(get_announcement))
        .route("/:id", put(update_announcement))
        .route("/:id", delete(delete_announcement))
        .route("/:id/read", post(mark_as_read))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
struct AnnouncementsQuery {
    category: Option<String>,
    page: Option<i64>,
    limit: Option<i64>,
}

async fn get_user_complex(state: &AppState, user_id: Uuid) -> AppResult<Uuid> {
    let complex: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT DISTINCT c.id
        FROM complexes c
        JOIN apartments a ON a.complex_id = c.id
        WHERE a.owner_id = $1 OR a.resident_id = $1
        LIMIT 1
        "#,
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;

    complex.map(|(id,)| id).ok_or_else(|| AppError::Forbidden)
}

/// Получить список объявлений
#[utoipa::path(
    get,
    path = "/api/v1/announcements",
    tag = "announcements",
    security(("bearer_auth" = [])),
    params(
        ("category" = Option<String>, Query, description = "Категория"),
        ("page" = Option<i64>, Query, description = "Номер страницы"),
        ("limit" = Option<i64>, Query, description = "Количество записей")
    ),
    responses(
        (status = 200, description = "Список объявлений", body = Vec<AnnouncementResponse>),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn list_announcements(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<AnnouncementsQuery>,
) -> AppResult<Json<Vec<AnnouncementResponse>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.page.unwrap_or(0) * limit;

    let announcements = sqlx::query_as::<_, Announcement>(
        r#"
        SELECT * FROM announcements
        WHERE complex_id = $1
          AND is_published = true
          AND (expires_at IS NULL OR expires_at > NOW())
        ORDER BY priority DESC, published_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(complex_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for ann in announcements {
        let author_name: Option<(String,)> = sqlx::query_as(
            "SELECT COALESCE(first_name || ' ' || last_name, 'Администратор') FROM users WHERE id = $1"
        )
        .bind(ann.author_id)
        .fetch_optional(&state.pool)
        .await?;

        let is_read: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM announcement_reads WHERE announcement_id = $1 AND user_id = $2",
        )
        .bind(ann.id)
        .bind(auth_user.user_id)
        .fetch_optional(&state.pool)
        .await?;

        response.push(AnnouncementResponse {
            id: ann.id,
            title: ann.title,
            content: ann.content,
            category: ann.category,
            priority: ann.priority,
            image_url: ann.image_url,
            author_name: author_name.map(|(n,)| n),
            views_count: ann.views_count,
            is_read: is_read.is_some(),
            published_at: ann.published_at,
            created_at: ann.created_at,
        });
    }

    Ok(Json(response))
}

/// Получить объявление по ID
#[utoipa::path(
    get,
    path = "/api/v1/announcements/{id}",
    tag = "announcements",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID объявления")
    ),
    responses(
        (status = 200, description = "Объявление", body = AnnouncementResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Не найдено")
    )
)]
pub async fn get_announcement(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<AnnouncementResponse>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let ann = sqlx::query_as::<_, Announcement>(
        "SELECT * FROM announcements WHERE id = $1 AND complex_id = $2",
    )
    .bind(id)
    .bind(complex_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Объявление не найдено".to_string()))?;

    sqlx::query("UPDATE announcements SET views_count = views_count + 1 WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    sqlx::query(
        r#"
        INSERT INTO announcement_reads (announcement_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (announcement_id, user_id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(auth_user.user_id)
    .execute(&state.pool)
    .await?;

    let author_name: Option<(String,)> = sqlx::query_as(
        "SELECT COALESCE(first_name || ' ' || last_name, 'Администратор') FROM users WHERE id = $1",
    )
    .bind(ann.author_id)
    .fetch_optional(&state.pool)
    .await?;

    Ok(Json(AnnouncementResponse {
        id: ann.id,
        title: ann.title,
        content: ann.content,
        category: ann.category,
        priority: ann.priority,
        image_url: ann.image_url,
        author_name: author_name.map(|(n,)| n),
        views_count: ann.views_count + 1,
        is_read: true,
        published_at: ann.published_at,
        created_at: ann.created_at,
    }))
}

/// Создать объявление
#[utoipa::path(
    post,
    path = "/api/v1/announcements",
    tag = "announcements",
    security(("bearer_auth" = [])),
    request_body = CreateAnnouncementRequest,
    responses(
        (status = 200, description = "Объявление создано", body = AnnouncementResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав")
    )
)]
pub async fn create_announcement(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateAnnouncementRequest>,
) -> AppResult<Json<AnnouncementResponse>> {
    let complex_id: Option<(Uuid,)> =
        sqlx::query_as("SELECT complex_id FROM osi WHERE chairman_id = $1")
            .bind(auth_user.user_id)
            .fetch_optional(&state.pool)
            .await?;

    let complex_id = complex_id.map(|(id,)| id).ok_or_else(|| {
        if is_chairman_or_higher(&auth_user.role) {
            AppError::BadRequest("complex_id требуется".to_string())
        } else {
            AppError::Forbidden
        }
    })?;

    let ann = sqlx::query_as::<_, Announcement>(
        r#"
        INSERT INTO announcements (
            complex_id, title, content, category, priority,
            image_url, expires_at, author_id, is_published, published_at
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, NOW())
        RETURNING *
        "#,
    )
    .bind(complex_id)
    .bind(&payload.title)
    .bind(&payload.content)
    .bind(payload.category.unwrap_or(AnnouncementCategory::General))
    .bind(payload.priority.unwrap_or(AnnouncementPriority::Normal))
    .bind(&payload.image_url)
    .bind(&payload.expires_at)
    .bind(auth_user.user_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(AnnouncementResponse {
        id: ann.id,
        title: ann.title,
        content: ann.content,
        category: ann.category,
        priority: ann.priority,
        image_url: ann.image_url,
        author_name: None,
        views_count: 0,
        is_read: true,
        published_at: ann.published_at,
        created_at: ann.created_at,
    }))
}

/// Обновить объявление
#[utoipa::path(
    put,
    path = "/api/v1/announcements/{id}",
    tag = "announcements",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID объявления")
    ),
    request_body = UpdateAnnouncementRequest,
    responses(
        (status = 200, description = "Объявление обновлено", body = AnnouncementResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "Не найдено")
    )
)]
pub async fn update_announcement(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateAnnouncementRequest>,
) -> AppResult<Json<AnnouncementResponse>> {
    let ann = sqlx::query_as::<_, Announcement>("SELECT * FROM announcements WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Объявление не найдено".to_string()))?;

    if ann.author_id != auth_user.user_id && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    let updated = sqlx::query_as::<_, Announcement>(
        r#"
        UPDATE announcements SET
            title = COALESCE($2, title),
            content = COALESCE($3, content),
            category = COALESCE($4, category),
            priority = COALESCE($5, priority),
            image_url = COALESCE($6, image_url),
            is_published = COALESCE($7, is_published),
            expires_at = COALESCE($8, expires_at),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(id)
    .bind(&payload.title)
    .bind(&payload.content)
    .bind(&payload.category)
    .bind(&payload.priority)
    .bind(&payload.image_url)
    .bind(&payload.is_published)
    .bind(&payload.expires_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(AnnouncementResponse {
        id: updated.id,
        title: updated.title,
        content: updated.content,
        category: updated.category,
        priority: updated.priority,
        image_url: updated.image_url,
        author_name: None,
        views_count: updated.views_count,
        is_read: true,
        published_at: updated.published_at,
        created_at: updated.created_at,
    }))
}

/// Удалить объявление
#[utoipa::path(
    delete,
    path = "/api/v1/announcements/{id}",
    tag = "announcements",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID объявления")
    ),
    responses(
        (status = 200, description = "Объявление удалено", body = SuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Недостаточно прав"),
        (status = 404, description = "Не найдено")
    )
)]
pub async fn delete_announcement(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let ann = sqlx::query_as::<_, Announcement>("SELECT * FROM announcements WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Объявление не найдено".to_string()))?;

    if ann.author_id != auth_user.user_id && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    sqlx::query("DELETE FROM announcements WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({"success": true})))
}

/// Отметить объявление как прочитанное
#[utoipa::path(
    post,
    path = "/api/v1/announcements/{id}/read",
    tag = "announcements",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID объявления")
    ),
    responses(
        (status = 200, description = "Отмечено как прочитанное", body = SuccessResponse),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn mark_as_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    sqlx::query(
        r#"
        INSERT INTO announcement_reads (announcement_id, user_id)
        VALUES ($1, $2)
        ON CONFLICT (announcement_id, user_id) DO NOTHING
        "#,
    )
    .bind(id)
    .bind(auth_user.user_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({"success": true})))
}
