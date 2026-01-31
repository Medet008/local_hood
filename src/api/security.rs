use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{AppState, AuthUser};
use crate::models::{
    BarrierAccessLogResponse, BarrierEntryRequest, Camera, CameraResponse, CameraStreamResponse,
    CreateGuestAccessRequest, GuestAccessResponse, IntercomCallResponse,
};
use crate::services::{barrier_service::generate_qr_code_base64, BarrierService, SmsService};

/// Успешный ответ
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
    pub message: String,
}

/// Запрос на открытие домофона
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct OpenIntercomRequest {
    pub intercom_id: Option<Uuid>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        // Шлагбаум
        .route("/barrier/open", post(open_barrier))
        .route("/barrier/guest-access", post(create_guest_access))
        .route("/barrier/guests", get(get_active_guests))
        .route("/barrier/guests/:id", delete(cancel_guest_access))
        .route("/barrier/history", get(get_barrier_history))
        .route("/barrier/entry", post(process_entry))
        .route("/barrier/exit", post(process_exit))
        // Камеры
        .route("/cameras", get(get_cameras))
        .route("/cameras/:id/stream", get(get_camera_stream))
        // Домофон
        .route("/intercom/open", post(open_intercom))
        .route("/intercom/calls", get(get_intercom_calls))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
struct PaginationQuery {
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

/// Открыть шлагбаум
#[utoipa::path(
    post,
    path = "/api/v1/security/barrier/open",
    tag = "security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Шлагбаум открыт", body = SuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа")
    )
)]
pub async fn open_barrier(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Value>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    sqlx::query(
        r#"
        INSERT INTO barrier_access_logs (complex_id, user_id, action)
        VALUES ($1, $2, 'entry')
        "#,
    )
    .bind(complex_id)
    .bind(auth_user.user_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "message": "Шлагбаум открыт"
    })))
}

/// Создать гостевой доступ
#[utoipa::path(
    post,
    path = "/api/v1/security/barrier/guest-access",
    tag = "security",
    security(("bearer_auth" = [])),
    request_body = CreateGuestAccessRequest,
    responses(
        (status = 200, description = "Гостевой доступ создан", body = GuestAccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа")
    )
)]
pub async fn create_guest_access(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateGuestAccessRequest>,
) -> AppResult<Json<GuestAccessResponse>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let duration = payload.duration_minutes.unwrap_or(30).min(240);

    let sms_service = SmsService::new(state.config.clone());
    let barrier_service = BarrierService::new(sms_service);

    let access = barrier_service
        .create_guest_access(
            &state.pool,
            complex_id,
            auth_user.user_id,
            payload.guest_name,
            payload.guest_phone,
            payload.vehicle_number,
            duration,
        )
        .await?;

    let qr_data = format!("LOCALHOOD:{}", access.access_code);
    let qr_code_url = generate_qr_code_base64(&qr_data).ok();

    if let Some(ref qr_url) = qr_code_url {
        sqlx::query("UPDATE guest_access SET qr_code_url = $1 WHERE id = $2")
            .bind(qr_url)
            .bind(access.id)
            .execute(&state.pool)
            .await?;
    }

    Ok(Json(GuestAccessResponse {
        id: access.id,
        guest_name: access.guest_name,
        guest_phone: access.guest_phone,
        vehicle_number: access.vehicle_number,
        access_code: access.access_code,
        qr_code_url,
        duration_minutes: access.duration_minutes,
        expires_at: access.expires_at,
        entered_at: access.entered_at,
        exited_at: access.exited_at,
        status: access.status,
        created_at: access.created_at,
    }))
}

/// Получить активных гостей
#[utoipa::path(
    get,
    path = "/api/v1/security/barrier/guests",
    tag = "security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Список активных гостей", body = Vec<GuestAccessResponse>),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа")
    )
)]
pub async fn get_active_guests(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Vec<GuestAccessResponse>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let guests = BarrierService::get_active_guests(&state.pool, complex_id).await?;

    let response: Vec<GuestAccessResponse> =
        guests.into_iter().map(GuestAccessResponse::from).collect();
    Ok(Json(response))
}

/// Отменить гостевой доступ
#[utoipa::path(
    delete,
    path = "/api/v1/security/barrier/guests/{id}",
    tag = "security",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID гостевого доступа")
    ),
    responses(
        (status = 200, description = "Доступ отменён", body = SuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа")
    )
)]
pub async fn cancel_guest_access(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(access_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let sms_service = SmsService::new(state.config.clone());
    let barrier_service = BarrierService::new(sms_service);

    barrier_service
        .cancel_access(&state.pool, access_id, auth_user.user_id)
        .await?;

    Ok(Json(json!({"success": true})))
}

/// Получить историю проездов
#[utoipa::path(
    get,
    path = "/api/v1/security/barrier/history",
    tag = "security",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<i64>, Query, description = "Номер страницы"),
        ("limit" = Option<i64>, Query, description = "Количество записей")
    ),
    responses(
        (status = 200, description = "История проездов", body = Vec<BarrierAccessLogResponse>),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа")
    )
)]
pub async fn get_barrier_history(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> AppResult<Json<Vec<BarrierAccessLogResponse>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let limit = pagination.limit.unwrap_or(50).min(100);
    let offset = pagination.page.unwrap_or(0) * limit;

    let logs = sqlx::query_as::<
        _,
        (
            Uuid,
            crate::models::BarrierAction,
            Option<String>,
            Option<Uuid>,
            Option<Uuid>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, action, vehicle_number, user_id, guest_access_id, created_at
        FROM barrier_access_logs
        WHERE complex_id = $1
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(complex_id)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for (id, action, vehicle_number, user_id, guest_access_id, created_at) in logs {
        let user_name = if let Some(uid) = user_id {
            sqlx::query_as::<_, (String,)>(
                "SELECT COALESCE(first_name || ' ' || last_name, phone) FROM users WHERE id = $1",
            )
            .bind(uid)
            .fetch_optional(&state.pool)
            .await?
            .map(|(n,)| n)
        } else {
            None
        };

        let guest_name = if let Some(gid) = guest_access_id {
            sqlx::query_as::<_, (Option<String>,)>(
                "SELECT guest_name FROM guest_access WHERE id = $1",
            )
            .bind(gid)
            .fetch_optional(&state.pool)
            .await?
            .and_then(|(n,)| n)
        } else {
            None
        };

        response.push(BarrierAccessLogResponse {
            id,
            action,
            vehicle_number,
            user_name,
            guest_name,
            created_at,
        });
    }

    Ok(Json(response))
}

/// Зарегистрировать въезд по коду
#[utoipa::path(
    post,
    path = "/api/v1/security/barrier/entry",
    tag = "security",
    request_body = BarrierEntryRequest,
    responses(
        (status = 200, description = "Въезд зарегистрирован", body = SuccessResponse),
        (status = 400, description = "Неверный код")
    )
)]
pub async fn process_entry(
    State(state): State<AppState>,
    Json(payload): Json<BarrierEntryRequest>,
) -> AppResult<Json<Value>> {
    let access_code = payload
        .access_code
        .ok_or_else(|| AppError::BadRequest("access_code обязателен".to_string()))?;

    let sms_service = SmsService::new(state.config.clone());
    let barrier_service = BarrierService::new(sms_service);

    barrier_service
        .process_entry(
            &state.pool,
            &access_code,
            payload.vehicle_number.as_deref(),
            payload.barrier_id,
        )
        .await?;

    Ok(Json(json!({
        "success": true,
        "message": "Въезд зарегистрирован"
    })))
}

/// Зарегистрировать выезд по коду
#[utoipa::path(
    post,
    path = "/api/v1/security/barrier/exit",
    tag = "security",
    request_body = BarrierEntryRequest,
    responses(
        (status = 200, description = "Выезд зарегистрирован", body = SuccessResponse),
        (status = 400, description = "Неверный код")
    )
)]
pub async fn process_exit(
    State(state): State<AppState>,
    Json(payload): Json<BarrierEntryRequest>,
) -> AppResult<Json<Value>> {
    let access_code = payload
        .access_code
        .ok_or_else(|| AppError::BadRequest("access_code обязателен".to_string()))?;

    let sms_service = SmsService::new(state.config.clone());
    let barrier_service = BarrierService::new(sms_service);

    barrier_service
        .process_exit(&state.pool, &access_code, payload.barrier_id)
        .await?;

    Ok(Json(json!({
        "success": true,
        "message": "Выезд зарегистрирован"
    })))
}

/// Получить список камер
#[utoipa::path(
    get,
    path = "/api/v1/security/cameras",
    tag = "security",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Список камер", body = Vec<CameraResponse>),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа")
    )
)]
pub async fn get_cameras(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Vec<CameraResponse>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let cameras = sqlx::query_as::<_, Camera>(
        "SELECT * FROM cameras WHERE complex_id = $1 AND is_active = true ORDER BY name",
    )
    .bind(complex_id)
    .fetch_all(&state.pool)
    .await?;

    let response: Vec<CameraResponse> = cameras
        .into_iter()
        .map(|c| CameraResponse {
            id: c.id,
            name: c.name,
            location: c.location,
            is_active: c.is_active,
        })
        .collect();

    Ok(Json(response))
}

/// Получить URL потока камеры
#[utoipa::path(
    get,
    path = "/api/v1/security/cameras/{id}/stream",
    tag = "security",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID камеры")
    ),
    responses(
        (status = 200, description = "URL потока", body = CameraStreamResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Камера не найдена")
    )
)]
pub async fn get_camera_stream(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(camera_id): Path<Uuid>,
) -> AppResult<Json<CameraStreamResponse>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let camera = sqlx::query_as::<_, Camera>(
        "SELECT * FROM cameras WHERE id = $1 AND complex_id = $2 AND is_active = true",
    )
    .bind(camera_id)
    .bind(complex_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Камера не найдена".to_string()))?;

    let stream_url = camera
        .stream_url
        .ok_or_else(|| AppError::NotFound("URL потока не настроен".to_string()))?;

    Ok(Json(CameraStreamResponse {
        id: camera.id,
        name: camera.name,
        stream_url,
    }))
}

/// Открыть домофон
#[utoipa::path(
    post,
    path = "/api/v1/security/intercom/open",
    tag = "security",
    security(("bearer_auth" = [])),
    request_body = OpenIntercomRequest,
    responses(
        (status = 200, description = "Домофон открыт", body = SuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Домофон не найден")
    )
)]
pub async fn open_intercom(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<Value>,
) -> AppResult<Json<Value>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let intercom_id = payload["intercom_id"]
        .as_str()
        .and_then(|s| Uuid::parse_str(s).ok());

    if let Some(id) = intercom_id {
        let exists: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM intercoms WHERE id = $1 AND complex_id = $2 AND is_active = true",
        )
        .bind(id)
        .bind(complex_id)
        .fetch_optional(&state.pool)
        .await?;

        if exists.is_none() {
            return Err(AppError::NotFound("Домофон не найден".to_string()));
        }
    }

    Ok(Json(json!({
        "success": true,
        "message": "Домофон открыт"
    })))
}

/// Получить историю звонков домофона
#[utoipa::path(
    get,
    path = "/api/v1/security/intercom/calls",
    tag = "security",
    security(("bearer_auth" = [])),
    params(
        ("page" = Option<i64>, Query, description = "Номер страницы"),
        ("limit" = Option<i64>, Query, description = "Количество записей")
    ),
    responses(
        (status = 200, description = "История звонков", body = Vec<IntercomCallResponse>),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn get_intercom_calls(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(pagination): Query<PaginationQuery>,
) -> AppResult<Json<Vec<IntercomCallResponse>>> {
    let apartment_ids: Vec<(Uuid,)> =
        sqlx::query_as("SELECT id FROM apartments WHERE owner_id = $1 OR resident_id = $1")
            .bind(auth_user.user_id)
            .fetch_all(&state.pool)
            .await?;

    let ids: Vec<Uuid> = apartment_ids.into_iter().map(|(id,)| id).collect();

    if ids.is_empty() {
        return Ok(Json(vec![]));
    }

    let limit = pagination.limit.unwrap_or(50).min(100);
    let offset = pagination.page.unwrap_or(0) * limit;

    let calls = sqlx::query_as::<
        _,
        (
            Uuid,
            Uuid,
            crate::models::IntercomCallStatus,
            Option<i32>,
            Option<String>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT ic.id, ic.intercom_id, ic.status, ic.duration_seconds, ic.snapshot_url, ic.created_at
        FROM intercom_calls ic
        WHERE ic.apartment_id = ANY($1)
        ORDER BY ic.created_at DESC
        LIMIT $2 OFFSET $3
        "#,
    )
    .bind(&ids)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for (id, intercom_id, status, duration, snapshot, created_at) in calls {
        let intercom_name: (String,) = sqlx::query_as("SELECT name FROM intercoms WHERE id = $1")
            .bind(intercom_id)
            .fetch_one(&state.pool)
            .await?;

        response.push(IntercomCallResponse {
            id,
            intercom_name: intercom_name.0,
            status,
            duration_seconds: duration,
            snapshot_url: snapshot,
            created_at,
        });
    }

    Ok(Json(response))
}
