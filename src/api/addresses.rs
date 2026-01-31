use axum::{
    extract::{Query, State},
    routing::{get, post},
    Json, Router,
};

use crate::error::{AppError, AppResult};
use crate::middleware::{AppState, AuthUser};
use crate::models::{Address, AddressResponse, CreateAddressRequest, SearchAddressQuery};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", post(create_address))
        .route("/search", get(search_addresses))
}

async fn search_addresses(
    State(state): State<AppState>,
    Query(query): Query<SearchAddressQuery>,
) -> AppResult<Json<Vec<AddressResponse>>> {
    let search_pattern = format!("%{}%", query.query);

    let addresses = sqlx::query_as::<_, (uuid::Uuid, String, Option<String>, String, String, String)>(
        r#"
        SELECT
            a.id,
            a.city_id,
            a.district,
            a.street,
            a.building,
            c.name as city_name
        FROM addresses a
        JOIN cities c ON c.id = a.city_id
        WHERE a.city_id = $1
          AND (a.street ILIKE $2 OR a.building ILIKE $2)
        ORDER BY a.street, a.building
        LIMIT 20
        "#,
    )
    .bind(&query.city)
    .bind(&search_pattern)
    .fetch_all(&state.pool)
    .await?;

    let response: Vec<AddressResponse> = addresses
        .into_iter()
        .map(|(id, city_id, district, street, building, city_name)| {
            AddressResponse {
                id,
                city_id: city_id.clone(),
                city_name: Some(city_name.clone()),
                district,
                street: street.clone(),
                building: building.clone(),
                full_address: format!("г. {}, {}, {}", city_name, street, building),
            }
        })
        .collect();

    Ok(Json(response))
}

async fn create_address(
    State(state): State<AppState>,
    _auth_user: AuthUser,
    Json(payload): Json<CreateAddressRequest>,
) -> AppResult<Json<AddressResponse>> {
    // Проверяем, существует ли город
    let city_exists: Option<(String,)> = sqlx::query_as(
        "SELECT name FROM cities WHERE id = $1 AND is_active = true"
    )
    .bind(&payload.city_id)
    .fetch_optional(&state.pool)
    .await?;

    let city_name = city_exists
        .ok_or_else(|| AppError::NotFound("Город не найден".to_string()))?
        .0;

    // Проверяем, существует ли уже такой адрес
    let existing: Option<(uuid::Uuid,)> = sqlx::query_as(
        r#"
        SELECT id FROM addresses
        WHERE city_id = $1 AND street = $2 AND building = $3
        "#
    )
    .bind(&payload.city_id)
    .bind(&payload.street)
    .bind(&payload.building)
    .fetch_optional(&state.pool)
    .await?;

    if let Some((id,)) = existing {
        return Ok(Json(AddressResponse {
            id,
            city_id: payload.city_id.clone(),
            city_name: Some(city_name.clone()),
            district: payload.district.clone(),
            street: payload.street.clone(),
            building: payload.building.clone(),
            full_address: format!("г. {}, {}, {}", city_name, payload.street, payload.building),
        }));
    }

    // Создаём адрес
    let address = sqlx::query_as::<_, Address>(
        r#"
        INSERT INTO addresses (city_id, district, street, building, postal_code, latitude, longitude)
        VALUES ($1, $2, $3, $4, $5, $6, $7)
        RETURNING *
        "#,
    )
    .bind(&payload.city_id)
    .bind(&payload.district)
    .bind(&payload.street)
    .bind(&payload.building)
    .bind(&payload.postal_code)
    .bind(&payload.latitude)
    .bind(&payload.longitude)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(AddressResponse {
        id: address.id,
        city_id: address.city_id,
        city_name: Some(city_name.clone()),
        district: address.district,
        street: address.street.clone(),
        building: address.building.clone(),
        full_address: format!("г. {}, {}, {}", city_name, address.street, address.building),
    }))
}
