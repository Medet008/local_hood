use axum::{extract::State, routing::get, Json, Router};

use crate::error::AppResult;
use crate::middleware::AppState;
use crate::models::{City, CityResponse};

pub fn routes() -> Router<AppState> {
    Router::new().route("/", get(list_cities))
}

/// Получение списка городов
#[utoipa::path(
    get,
    path = "/api/v1/cities",
    tag = "cities",
    responses(
        (status = 200, description = "Список городов", body = Vec<CityResponse>)
    )
)]
pub async fn list_cities(State(state): State<AppState>) -> AppResult<Json<Vec<CityResponse>>> {
    let cities =
        sqlx::query_as::<_, City>("SELECT * FROM cities WHERE is_active = true ORDER BY name")
            .fetch_all(&state.pool)
            .await?;

    let response: Vec<CityResponse> = cities.into_iter().map(CityResponse::from).collect();
    Ok(Json(response))
}
