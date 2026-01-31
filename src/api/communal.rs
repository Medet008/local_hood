use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::NaiveDate;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{AppState, AuthUser};
use crate::models::{
    Bill, BillItem, BillItemResponse, BillResponse, CreatePaymentRequest, Meter, MeterReading,
    MeterResponse, PaymentResponse, PaymentStatus, SubmitReadingRequest,
};

/// Ответ на подачу показаний
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SubmitReadingResponse {
    pub success: bool,
    pub consumption: Option<rust_decimal::Decimal>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/meters", get(get_meters))
        .route("/meters/readings", post(submit_reading))
        .route("/meters/readings/history", get(get_readings_history))
        .route("/bills", get(get_bills))
        .route("/bills/:id", get(get_bill))
        .route("/payments", post(create_payment))
        .route("/payments/:id", get(get_payment))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct BillsQuery {
    pub apartment_id: Option<Uuid>,
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

async fn get_user_apartments(state: &AppState, user_id: Uuid) -> AppResult<Vec<Uuid>> {
    let apartments: Vec<(Uuid,)> =
        sqlx::query_as("SELECT id FROM apartments WHERE owner_id = $1 OR resident_id = $1")
            .bind(user_id)
            .fetch_all(&state.pool)
            .await?;

    if apartments.is_empty() {
        return Err(AppError::Forbidden);
    }

    Ok(apartments.into_iter().map(|(id,)| id).collect())
}

/// Получить счётчики
#[utoipa::path(
    get,
    path = "/api/v1/communal/meters",
    tag = "communal",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Список счётчиков", body = Vec<MeterResponse>),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет квартир")
    )
)]
pub async fn get_meters(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Vec<MeterResponse>>> {
    let apartment_ids = get_user_apartments(&state, auth_user.user_id).await?;

    let meters = sqlx::query_as::<_, Meter>(
        "SELECT * FROM meters WHERE apartment_id = ANY($1) AND is_active = true ORDER BY utility_type"
    )
    .bind(&apartment_ids)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for meter in meters {
        let last_reading: Option<(rust_decimal::Decimal, NaiveDate)> = sqlx::query_as(
            r#"
            SELECT value, reading_date
            FROM meter_readings
            WHERE meter_id = $1
            ORDER BY reading_date DESC
            LIMIT 1
            "#,
        )
        .bind(meter.id)
        .fetch_optional(&state.pool)
        .await?;

        response.push(MeterResponse {
            id: meter.id,
            utility_type: meter.utility_type,
            serial_number: meter.serial_number,
            last_reading: last_reading.as_ref().map(|(v, _)| *v),
            last_reading_date: last_reading.map(|(_, d)| d),
            is_active: meter.is_active,
        });
    }

    Ok(Json(response))
}

/// Подать показания счётчика
#[utoipa::path(
    post,
    path = "/api/v1/communal/meters/readings",
    tag = "communal",
    security(("bearer_auth" = [])),
    request_body = SubmitReadingRequest,
    responses(
        (status = 200, description = "Показания приняты", body = SubmitReadingResponse),
        (status = 400, description = "Показание меньше предыдущего"),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа"),
        (status = 404, description = "Счётчик не найден")
    )
)]
pub async fn submit_reading(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<SubmitReadingRequest>,
) -> AppResult<Json<Value>> {
    let meter = sqlx::query_as::<_, Meter>("SELECT * FROM meters WHERE id = $1")
        .bind(payload.meter_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Счетчик не найден".to_string()))?;

    let apartment_ids = get_user_apartments(&state, auth_user.user_id).await?;
    if !apartment_ids.contains(&meter.apartment_id) {
        return Err(AppError::Forbidden);
    }

    let previous: Option<(rust_decimal::Decimal,)> = sqlx::query_as(
        r#"
        SELECT value FROM meter_readings
        WHERE meter_id = $1
        ORDER BY reading_date DESC
        LIMIT 1
        "#,
    )
    .bind(payload.meter_id)
    .fetch_optional(&state.pool)
    .await?;

    let previous_value = previous.map(|(v,)| v);
    let consumption = previous_value.map(|pv| payload.value - pv);

    if let Some(pv) = previous_value {
        if payload.value < pv {
            return Err(AppError::BadRequest(
                "Показание не может быть меньше предыдущего".to_string(),
            ));
        }
    }

    let today = chrono::Utc::now().date_naive();

    sqlx::query(
        r#"
        INSERT INTO meter_readings (meter_id, apartment_id, value, previous_value, consumption, reading_date, submitted_by, photo_url)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        "#
    )
    .bind(payload.meter_id)
    .bind(meter.apartment_id)
    .bind(payload.value)
    .bind(previous_value)
    .bind(consumption)
    .bind(today)
    .bind(auth_user.user_id)
    .bind(&payload.photo_url)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "consumption": consumption
    })))
}

/// Получить историю показаний счётчика
#[utoipa::path(
    get,
    path = "/api/v1/communal/meters/readings/history",
    tag = "communal",
    security(("bearer_auth" = [])),
    params(
        ("meter_id" = Uuid, Query, description = "ID счётчика")
    ),
    responses(
        (status = 200, description = "История показаний", body = Vec<MeterReading>),
        (status = 400, description = "meter_id обязателен"),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа"),
        (status = 404, description = "Счётчик не найден")
    )
)]
pub async fn get_readings_history(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> AppResult<Json<Vec<MeterReading>>> {
    let meter_id = params
        .get("meter_id")
        .ok_or_else(|| AppError::BadRequest("meter_id обязателен".to_string()))?;

    let meter_uuid = Uuid::parse_str(meter_id)
        .map_err(|_| AppError::BadRequest("Неверный формат meter_id".to_string()))?;

    let meter = sqlx::query_as::<_, Meter>("SELECT * FROM meters WHERE id = $1")
        .bind(meter_uuid)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Счетчик не найден".to_string()))?;

    let apartment_ids = get_user_apartments(&state, auth_user.user_id).await?;
    if !apartment_ids.contains(&meter.apartment_id) {
        return Err(AppError::Forbidden);
    }

    let readings = sqlx::query_as::<_, MeterReading>(
        r#"
        SELECT * FROM meter_readings
        WHERE meter_id = $1
        ORDER BY reading_date DESC
        LIMIT 12
        "#,
    )
    .bind(meter_uuid)
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(readings))
}

/// Получить счета
#[utoipa::path(
    get,
    path = "/api/v1/communal/bills",
    tag = "communal",
    security(("bearer_auth" = [])),
    params(
        ("apartment_id" = Option<Uuid>, Query, description = "ID квартиры"),
        ("status" = Option<String>, Query, description = "Статус счёта"),
        ("page" = Option<i64>, Query, description = "Номер страницы"),
        ("limit" = Option<i64>, Query, description = "Количество записей")
    ),
    responses(
        (status = 200, description = "Список счетов", body = Vec<BillResponse>),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет квартир")
    )
)]
pub async fn get_bills(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<BillsQuery>,
) -> AppResult<Json<Vec<BillResponse>>> {
    let apartment_ids = get_user_apartments(&state, auth_user.user_id).await?;

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.page.unwrap_or(0) * limit;

    let bills = sqlx::query_as::<_, Bill>(
        r#"
        SELECT * FROM bills
        WHERE apartment_id = ANY($1)
          AND ($2::uuid IS NULL OR apartment_id = $2)
          AND ($3::varchar IS NULL OR status::text = $3)
        ORDER BY period_end DESC
        LIMIT $4 OFFSET $5
        "#,
    )
    .bind(&apartment_ids)
    .bind(&query.apartment_id)
    .bind(&query.status)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for bill in bills {
        let items = sqlx::query_as::<_, BillItem>("SELECT * FROM bill_items WHERE bill_id = $1")
            .bind(bill.id)
            .fetch_all(&state.pool)
            .await?;

        response.push(BillResponse {
            id: bill.id,
            period: format!("{} - {}", bill.period_start, bill.period_end),
            amount: bill.amount,
            debt: bill.debt,
            penalty: bill.penalty,
            total_amount: bill.total_amount,
            status: bill.status,
            due_date: bill.due_date,
            items: items
                .into_iter()
                .map(|i| BillItemResponse {
                    utility_type: i.utility_type,
                    description: i.description,
                    quantity: i.quantity,
                    unit: i.unit,
                    rate: i.rate,
                    amount: i.amount,
                })
                .collect(),
        });
    }

    Ok(Json(response))
}

/// Получить счёт по ID
#[utoipa::path(
    get,
    path = "/api/v1/communal/bills/{id}",
    tag = "communal",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID счёта")
    ),
    responses(
        (status = 200, description = "Счёт", body = BillResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Счёт не найден")
    )
)]
pub async fn get_bill(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<BillResponse>> {
    let apartment_ids = get_user_apartments(&state, auth_user.user_id).await?;

    let bill =
        sqlx::query_as::<_, Bill>("SELECT * FROM bills WHERE id = $1 AND apartment_id = ANY($2)")
            .bind(id)
            .bind(&apartment_ids)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Счёт не найден".to_string()))?;

    let items = sqlx::query_as::<_, BillItem>("SELECT * FROM bill_items WHERE bill_id = $1")
        .bind(bill.id)
        .fetch_all(&state.pool)
        .await?;

    Ok(Json(BillResponse {
        id: bill.id,
        period: format!("{} - {}", bill.period_start, bill.period_end),
        amount: bill.amount,
        debt: bill.debt,
        penalty: bill.penalty,
        total_amount: bill.total_amount,
        status: bill.status,
        due_date: bill.due_date,
        items: items
            .into_iter()
            .map(|i| BillItemResponse {
                utility_type: i.utility_type,
                description: i.description,
                quantity: i.quantity,
                unit: i.unit,
                rate: i.rate,
                amount: i.amount,
            })
            .collect(),
    }))
}

/// Создать платёж
#[utoipa::path(
    post,
    path = "/api/v1/communal/payments",
    tag = "communal",
    security(("bearer_auth" = [])),
    request_body = CreatePaymentRequest,
    responses(
        (status = 200, description = "Платёж создан", body = PaymentResponse),
        (status = 400, description = "Счёт уже оплачен"),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Счёт не найден")
    )
)]
pub async fn create_payment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreatePaymentRequest>,
) -> AppResult<Json<PaymentResponse>> {
    let apartment_ids = get_user_apartments(&state, auth_user.user_id).await?;

    let bill =
        sqlx::query_as::<_, Bill>("SELECT * FROM bills WHERE id = $1 AND apartment_id = ANY($2)")
            .bind(payload.bill_id)
            .bind(&apartment_ids)
            .fetch_optional(&state.pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Счёт не найден".to_string()))?;

    if bill.status == crate::models::BillStatus::Paid {
        return Err(AppError::BadRequest("Счёт уже оплачен".to_string()));
    }

    let payment = sqlx::query_as::<_, crate::models::Payment>(
        r#"
        INSERT INTO payments (bill_id, apartment_id, user_id, amount, method, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#,
    )
    .bind(payload.bill_id)
    .bind(bill.apartment_id)
    .bind(auth_user.user_id)
    .bind(bill.total_amount)
    .bind(&payload.method)
    .bind(PaymentStatus::Pending)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(PaymentResponse {
        id: payment.id,
        amount: payment.amount,
        method: payment.method,
        status: payment.status,
        payment_url: payment.payment_url,
        created_at: payment.created_at,
    }))
}

/// Получить платёж по ID
#[utoipa::path(
    get,
    path = "/api/v1/communal/payments/{id}",
    tag = "communal",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID платежа")
    ),
    responses(
        (status = 200, description = "Платёж", body = PaymentResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Платёж не найден")
    )
)]
pub async fn get_payment(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<PaymentResponse>> {
    let payment = sqlx::query_as::<_, crate::models::Payment>(
        "SELECT * FROM payments WHERE id = $1 AND user_id = $2",
    )
    .bind(id)
    .bind(auth_user.user_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Платеж не найден".to_string()))?;

    Ok(Json(PaymentResponse {
        id: payment.id,
        amount: payment.amount,
        method: payment.method,
        status: payment.status,
        payment_url: payment.payment_url,
        created_at: payment.created_at,
    }))
}
