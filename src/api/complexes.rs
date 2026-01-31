use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{AppState, AuthUser};
use crate::models::{
    Complex, ComplexAmenities, ComplexResponse, ComplexStatus, CreateComplexRequest,
    JoinComplexRequest, JoinRequestStatus, SearchComplexQuery,
};

/// Ответ на проверку существования ЖК
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ComplexExistsResponse {
    pub exists: bool,
    pub complex_id: Option<Uuid>,
    pub complex_name: Option<String>,
}

/// Ответ на подачу заявки
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct JoinComplexResponse {
    pub success: bool,
    pub request_id: Uuid,
    pub message: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_complex))
        .route("/search", get(search_complexes))
        .route("/check", get(check_complex_exists))
        .route("/:id", get(get_complex))
        .route("/:id/join", post(join_complex))
}

/// Поиск жилых комплексов
#[utoipa::path(
    get,
    path = "/api/v1/complexes/search",
    tag = "complexes",
    params(
        ("city" = Option<String>, Query, description = "ID города"),
        ("query" = Option<String>, Query, description = "Поисковый запрос")
    ),
    responses(
        (status = 200, description = "Список ЖК", body = Vec<ComplexResponse>)
    )
)]
pub async fn search_complexes(
    State(state): State<AppState>,
    Query(query): Query<SearchComplexQuery>,
) -> AppResult<Json<Vec<ComplexResponse>>> {
    let search_pattern = query.query.as_ref().map(|q| format!("%{}%", q));

    let complexes = sqlx::query_as::<_, Complex>(
        r#"
        SELECT * FROM complexes
        WHERE ($1::varchar IS NULL OR city_id = $1)
          AND ($2::varchar IS NULL OR name ILIKE $2)
          AND status = 'active'
        ORDER BY name
        LIMIT 50
        "#,
    )
    .bind(&query.city)
    .bind(&search_pattern)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for complex in complexes {
        // Получаем адрес
        let address: Option<String> = if let Some(addr_id) = complex.address_id {
            sqlx::query_as::<_, (String, String, String)>(
                r#"
                SELECT c.name, a.street, a.building
                FROM addresses a
                JOIN cities c ON c.id = a.city_id
                WHERE a.id = $1
                "#,
            )
            .bind(addr_id)
            .fetch_optional(&state.pool)
            .await?
            .map(|(city, street, building)| format!("г. {}, {}, {}", city, street, building))
        } else {
            None
        };

        // Получаем фото
        let photos: Vec<(String,)> = sqlx::query_as(
            "SELECT url FROM complex_photos WHERE complex_id = $1 ORDER BY sort_order",
        )
        .bind(complex.id)
        .fetch_all(&state.pool)
        .await?;

        response.push(ComplexResponse {
            id: complex.id,
            city_id: complex.city_id,
            name: complex.name,
            description: complex.description,
            address,
            buildings_count: complex.buildings_count,
            floors_count: complex.floors_count,
            apartments_count: complex.apartments_count,
            year_built: complex.year_built,
            amenities: ComplexAmenities {
                has_parking: complex.has_parking,
                has_underground_parking: complex.has_underground_parking,
                has_playground: complex.has_playground,
                has_gym: complex.has_gym,
                has_concierge: complex.has_concierge,
                has_security: complex.has_security,
                has_cctv: complex.has_cctv,
            },
            status: complex.status,
            photos: photos.into_iter().map(|(url,)| url).collect(),
        });
    }

    Ok(Json(response))
}

/// Получение ЖК по ID
#[utoipa::path(
    get,
    path = "/api/v1/complexes/{id}",
    tag = "complexes",
    params(
        ("id" = Uuid, Path, description = "ID жилого комплекса")
    ),
    responses(
        (status = 200, description = "Информация о ЖК", body = ComplexResponse),
        (status = 404, description = "ЖК не найден")
    )
)]
pub async fn get_complex(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> AppResult<Json<ComplexResponse>> {
    let complex = sqlx::query_as::<_, Complex>("SELECT * FROM complexes WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("ЖК не найден".to_string()))?;

    let address: Option<String> = if let Some(addr_id) = complex.address_id {
        sqlx::query_as::<_, (String, String, String)>(
            r#"
            SELECT c.name, a.street, a.building
            FROM addresses a
            JOIN cities c ON c.id = a.city_id
            WHERE a.id = $1
            "#,
        )
        .bind(addr_id)
        .fetch_optional(&state.pool)
        .await?
        .map(|(city, street, building)| format!("г. {}, {}, {}", city, street, building))
    } else {
        None
    };

    let photos: Vec<(String,)> =
        sqlx::query_as("SELECT url FROM complex_photos WHERE complex_id = $1 ORDER BY sort_order")
            .bind(complex.id)
            .fetch_all(&state.pool)
            .await?;

    Ok(Json(ComplexResponse {
        id: complex.id,
        city_id: complex.city_id,
        name: complex.name,
        description: complex.description,
        address,
        buildings_count: complex.buildings_count,
        floors_count: complex.floors_count,
        apartments_count: complex.apartments_count,
        year_built: complex.year_built,
        amenities: ComplexAmenities {
            has_parking: complex.has_parking,
            has_underground_parking: complex.has_underground_parking,
            has_playground: complex.has_playground,
            has_gym: complex.has_gym,
            has_concierge: complex.has_concierge,
            has_security: complex.has_security,
            has_cctv: complex.has_cctv,
        },
        status: complex.status,
        photos: photos.into_iter().map(|(url,)| url).collect(),
    }))
}

/// Проверка существования ЖК по адресу
#[utoipa::path(
    get,
    path = "/api/v1/complexes/check",
    tag = "complexes",
    params(
        ("address_id" = Uuid, Query, description = "ID адреса")
    ),
    responses(
        (status = 200, description = "Результат проверки", body = ComplexExistsResponse),
        (status = 400, description = "address_id обязателен")
    )
)]
pub async fn check_complex_exists(
    State(state): State<AppState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> AppResult<Json<Value>> {
    let address_id = params
        .get("address_id")
        .ok_or_else(|| AppError::BadRequest("address_id обязателен".to_string()))?;

    let address_uuid = Uuid::parse_str(address_id)
        .map_err(|_| AppError::BadRequest("Неверный формат address_id".to_string()))?;

    let existing: Option<(Uuid, String)> =
        sqlx::query_as("SELECT id, name FROM complexes WHERE address_id = $1")
            .bind(address_uuid)
            .fetch_optional(&state.pool)
            .await?;

    match existing {
        Some((id, name)) => Ok(Json(json!({
            "exists": true,
            "complex_id": id,
            "complex_name": name
        }))),
        None => Ok(Json(json!({
            "exists": false
        }))),
    }
}

/// Создание нового ЖК
#[utoipa::path(
    post,
    path = "/api/v1/complexes",
    tag = "complexes",
    security(("bearer_auth" = [])),
    request_body = CreateComplexRequest,
    responses(
        (status = 200, description = "ЖК создан", body = ComplexResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Город не найден")
    )
)]
pub async fn create_complex(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateComplexRequest>,
) -> AppResult<Json<ComplexResponse>> {
    // Проверяем город
    let city_exists: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM cities WHERE id = $1 AND is_active = true")
            .bind(&payload.city_id)
            .fetch_optional(&state.pool)
            .await?;

    if city_exists.is_none() {
        return Err(AppError::NotFound("Город не найден".to_string()));
    }

    let complex = sqlx::query_as::<_, Complex>(
        r#"
        INSERT INTO complexes (
            city_id, address_id, name, description,
            buildings_count, floors_count, apartments_count, year_built,
            has_parking, has_underground_parking, has_playground,
            has_gym, has_concierge, has_security, has_cctv,
            status, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17)
        RETURNING *
        "#,
    )
    .bind(&payload.city_id)
    .bind(&payload.address_id)
    .bind(&payload.name)
    .bind(&payload.description)
    .bind(&payload.buildings_count)
    .bind(&payload.floors_count)
    .bind(&payload.apartments_count)
    .bind(&payload.year_built)
    .bind(payload.has_parking.unwrap_or(false))
    .bind(payload.has_underground_parking.unwrap_or(false))
    .bind(payload.has_playground.unwrap_or(false))
    .bind(payload.has_gym.unwrap_or(false))
    .bind(payload.has_concierge.unwrap_or(false))
    .bind(payload.has_security.unwrap_or(false))
    .bind(payload.has_cctv.unwrap_or(false))
    .bind(ComplexStatus::Pending)
    .bind(auth_user.user_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(ComplexResponse {
        id: complex.id,
        city_id: complex.city_id,
        name: complex.name,
        description: complex.description,
        address: None,
        buildings_count: complex.buildings_count,
        floors_count: complex.floors_count,
        apartments_count: complex.apartments_count,
        year_built: complex.year_built,
        amenities: ComplexAmenities {
            has_parking: complex.has_parking,
            has_underground_parking: complex.has_underground_parking,
            has_playground: complex.has_playground,
            has_gym: complex.has_gym,
            has_concierge: complex.has_concierge,
            has_security: complex.has_security,
            has_cctv: complex.has_cctv,
        },
        status: complex.status,
        photos: vec![],
    }))
}

/// Подача заявки на присоединение к ЖК
#[utoipa::path(
    post,
    path = "/api/v1/complexes/{id}/join",
    tag = "complexes",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID жилого комплекса")
    ),
    request_body = JoinComplexRequest,
    responses(
        (status = 200, description = "Заявка отправлена", body = JoinComplexResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "ЖК не найден"),
        (status = 409, description = "Заявка уже существует")
    )
)]
pub async fn join_complex(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(complex_id): Path<Uuid>,
    Json(payload): Json<JoinComplexRequest>,
) -> AppResult<Json<Value>> {
    // Проверяем ЖК
    let complex_exists: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM complexes WHERE id = $1 AND status = 'active'")
            .bind(complex_id)
            .fetch_optional(&state.pool)
            .await?;

    if complex_exists.is_none() {
        return Err(AppError::NotFound("ЖК не найден".to_string()));
    }

    // Проверяем, нет ли уже активной заявки
    let existing_request: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT id FROM join_requests
        WHERE user_id = $1 AND complex_id = $2 AND status = 'pending'
        "#,
    )
    .bind(auth_user.user_id)
    .bind(complex_id)
    .fetch_optional(&state.pool)
    .await?;

    if existing_request.is_some() {
        return Err(AppError::Conflict(
            "У вас уже есть активная заявка".to_string(),
        ));
    }

    // Создаём заявку
    let request_id: (Uuid,) = sqlx::query_as(
        r#"
        INSERT INTO join_requests (user_id, complex_id, apartment_number, building, is_owner, status)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING id
        "#,
    )
    .bind(auth_user.user_id)
    .bind(complex_id)
    .bind(&payload.apartment_number)
    .bind(&payload.building)
    .bind(payload.is_owner)
    .bind(JoinRequestStatus::Pending)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "request_id": request_id.0,
        "message": "Заявка отправлена на рассмотрение"
    })))
}
