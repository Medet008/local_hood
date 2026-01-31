use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{is_chairman_or_higher, AppState, AuthUser};
use crate::models::{
    AddMaintenanceCommentRequest, CreateMaintenanceRequest, MaintenanceComment,
    MaintenancePhoto, MaintenancePhotoResponse, MaintenancePriority,
    MaintenanceRequest, MaintenanceRequestResponse, MaintenanceStatus,
    RateMaintenanceRequest, UpdateMaintenanceStatusRequest,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_requests))
        .route("/", post(create_request))
        .route("/:id", get(get_request))
        .route("/:id/status", put(update_status))
        .route("/:id/rate", post(rate_request))
        .route("/:id/comments", get(get_comments))
        .route("/:id/comments", post(add_comment))
}

#[derive(Debug, Deserialize)]
struct RequestsQuery {
    status: Option<String>,
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
        "#
    )
    .bind(user_id)
    .fetch_optional(&state.pool)
    .await?;

    complex.map(|(id,)| id).ok_or_else(|| AppError::Forbidden)
}

async fn list_requests(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<RequestsQuery>,
) -> AppResult<Json<Vec<MaintenanceRequestResponse>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.page.unwrap_or(0) * limit;

    let requests = sqlx::query_as::<_, MaintenanceRequest>(
        r#"
        SELECT * FROM maintenance_requests
        WHERE complex_id = $1
          AND ($2::varchar IS NULL OR status::text = $2)
          AND ($3::varchar IS NULL OR category::text = $3)
        ORDER BY
            CASE priority
                WHEN 'emergency' THEN 1
                WHEN 'high' THEN 2
                WHEN 'normal' THEN 3
                ELSE 4
            END,
            created_at DESC
        LIMIT $4 OFFSET $5
        "#
    )
    .bind(complex_id)
    .bind(&query.status)
    .bind(&query.category)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for req in requests {
        response.push(build_request_response(&state, &req).await?);
    }

    Ok(Json(response))
}

async fn build_request_response(
    state: &AppState,
    req: &MaintenanceRequest,
) -> AppResult<MaintenanceRequestResponse> {
    let assigned_name: Option<String> = if let Some(worker_id) = req.assigned_to {
        sqlx::query_as::<_, (String, String)>(
            "SELECT first_name, last_name FROM osi_workers WHERE id = $1"
        )
        .bind(worker_id)
        .fetch_optional(&state.pool)
        .await?
        .map(|(f, l)| format!("{} {}", f, l))
    } else {
        None
    };

    let photos = sqlx::query_as::<_, MaintenancePhoto>(
        "SELECT * FROM maintenance_photos WHERE request_id = $1 ORDER BY is_before DESC"
    )
    .bind(req.id)
    .fetch_all(&state.pool)
    .await?;

    let comments_count: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM maintenance_comments WHERE request_id = $1"
    )
    .bind(req.id)
    .fetch_one(&state.pool)
    .await?;

    Ok(MaintenanceRequestResponse {
        id: req.id,
        category: req.category.clone(),
        title: req.title.clone(),
        description: req.description.clone(),
        location: req.location.clone(),
        priority: req.priority.clone(),
        status: req.status.clone(),
        assigned_to_name: assigned_name,
        photos: photos.into_iter().map(|p| MaintenancePhotoResponse {
            url: p.url,
            is_before: p.is_before,
        }).collect(),
        comments_count: comments_count.0 as i32,
        rating: req.rating,
        created_at: req.created_at,
    })
}

async fn get_request(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<MaintenanceRequestResponse>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let req = sqlx::query_as::<_, MaintenanceRequest>(
        "SELECT * FROM maintenance_requests WHERE id = $1 AND complex_id = $2"
    )
    .bind(id)
    .bind(complex_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Заявка не найдена".to_string()))?;

    let response = build_request_response(&state, &req).await?;
    Ok(Json(response))
}

async fn create_request(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateMaintenanceRequest>,
) -> AppResult<Json<MaintenanceRequestResponse>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let req = sqlx::query_as::<_, MaintenanceRequest>(
        r#"
        INSERT INTO maintenance_requests (
            complex_id, apartment_id, requester_id, category, title,
            description, location, priority, status
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
        RETURNING *
        "#
    )
    .bind(complex_id)
    .bind(&payload.apartment_id)
    .bind(auth_user.user_id)
    .bind(&payload.category)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(&payload.location)
    .bind(payload.priority.clone().unwrap_or(MaintenancePriority::Normal))
    .bind(MaintenanceStatus::New)
    .fetch_one(&state.pool)
    .await?;

    let response = build_request_response(&state, &req).await?;
    Ok(Json(response))
}

async fn update_status(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateMaintenanceStatusRequest>,
) -> AppResult<Json<MaintenanceRequestResponse>> {
    let req = sqlx::query_as::<_, MaintenanceRequest>(
        "SELECT * FROM maintenance_requests WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Заявка не найдена".to_string()))?;

    // Проверяем права (председатель или автор заявки для отмены)
    let is_chairman: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM osi WHERE complex_id = $1 AND chairman_id = $2"
    )
    .bind(req.complex_id)
    .bind(auth_user.user_id)
    .fetch_optional(&state.pool)
    .await?;

    let can_update = is_chairman.is_some()
        || is_chairman_or_higher(&auth_user.role)
        || (req.requester_id == auth_user.user_id && payload.status == MaintenanceStatus::Cancelled);

    if !can_update {
        return Err(AppError::Forbidden);
    }

    let completed_at = if payload.status == MaintenanceStatus::Completed {
        Some(chrono::Utc::now())
    } else {
        None
    };

    let updated = sqlx::query_as::<_, MaintenanceRequest>(
        r#"
        UPDATE maintenance_requests SET
            status = $2,
            completion_notes = COALESCE($3, completion_notes),
            completed_at = COALESCE($4, completed_at),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#
    )
    .bind(id)
    .bind(&payload.status)
    .bind(&payload.completion_notes)
    .bind(completed_at)
    .fetch_one(&state.pool)
    .await?;

    let response = build_request_response(&state, &updated).await?;
    Ok(Json(response))
}

async fn rate_request(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<RateMaintenanceRequest>,
) -> AppResult<Json<Value>> {
    let req = sqlx::query_as::<_, MaintenanceRequest>(
        "SELECT * FROM maintenance_requests WHERE id = $1"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Заявка не найдена".to_string()))?;

    if req.requester_id != auth_user.user_id {
        return Err(AppError::Forbidden);
    }

    if req.status != MaintenanceStatus::Completed {
        return Err(AppError::BadRequest("Можно оценить только завершённую заявку".to_string()));
    }

    if payload.rating < 1 || payload.rating > 5 {
        return Err(AppError::BadRequest("Оценка должна быть от 1 до 5".to_string()));
    }

    sqlx::query(
        "UPDATE maintenance_requests SET rating = $2, rating_comment = $3, updated_at = NOW() WHERE id = $1"
    )
    .bind(id)
    .bind(payload.rating)
    .bind(&payload.comment)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({"success": true})))
}

async fn get_comments(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Vec<Value>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    // Проверяем доступ
    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM maintenance_requests WHERE id = $1 AND complex_id = $2"
    )
    .bind(id)
    .bind(complex_id)
    .fetch_optional(&state.pool)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("Заявка не найдена".to_string()));
    }

    let comments = sqlx::query_as::<_, MaintenanceComment>(
        "SELECT * FROM maintenance_comments WHERE request_id = $1 ORDER BY created_at"
    )
    .bind(id)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for comment in comments {
        let author: (String,) = sqlx::query_as(
            "SELECT COALESCE(first_name || ' ' || last_name, phone) FROM users WHERE id = $1"
        )
        .bind(comment.user_id)
        .fetch_one(&state.pool)
        .await?;

        response.push(json!({
            "id": comment.id,
            "content": comment.content,
            "author": author.0,
            "created_at": comment.created_at
        }));
    }

    Ok(Json(response))
}

async fn add_comment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<AddMaintenanceCommentRequest>,
) -> AppResult<Json<Value>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let exists: Option<(i32,)> = sqlx::query_as(
        "SELECT 1 FROM maintenance_requests WHERE id = $1 AND complex_id = $2"
    )
    .bind(id)
    .bind(complex_id)
    .fetch_optional(&state.pool)
    .await?;

    if exists.is_none() {
        return Err(AppError::NotFound("Заявка не найдена".to_string()));
    }

    let comment_id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO maintenance_comments (request_id, user_id, content)
        VALUES ($1, $2, $3)
        RETURNING id
        "#
    )
    .bind(id)
    .bind(auth_user.user_id)
    .bind(&payload.content)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "comment_id": comment_id.0
    })))
}
