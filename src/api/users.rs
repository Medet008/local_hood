use axum::{
    extract::{Multipart, State},
    routing::{get, post, put},
    Json, Router,
};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::middleware::{AppState, AuthUser};
use crate::models::{ApartmentResponse, UpdateUserRequest, User, UserPublic};
use crate::services::{
    file_service::{validate_image_content_type, MAX_IMAGE_SIZE},
    AuthService, FileService,
};

/// Ответ на загрузку аватара
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct AvatarUploadResponse {
    pub success: bool,
    pub avatar_url: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/me", get(get_me))
        .route("/me", put(update_me))
        .route("/me/avatar", post(upload_avatar))
        .route("/me/apartments", get(get_my_apartments))
}

/// Получение профиля текущего пользователя
#[utoipa::path(
    get,
    path = "/api/v1/users/me",
    tag = "users",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Профиль пользователя", body = UserPublic),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn get_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<UserPublic>> {
    let user = AuthService::get_user_by_id(&state.pool, auth_user.user_id).await?;
    Ok(Json(UserPublic::from(user)))
}

/// Обновление профиля текущего пользователя
#[utoipa::path(
    put,
    path = "/api/v1/users/me",
    tag = "users",
    security(("bearer_auth" = [])),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "Профиль обновлён", body = UserPublic),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn update_me(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<UpdateUserRequest>,
) -> AppResult<Json<UserPublic>> {
    let user = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET
            first_name = COALESCE($2, first_name),
            last_name = COALESCE($3, last_name),
            middle_name = COALESCE($4, middle_name),
            email = COALESCE($5, email),
            updated_at = NOW()
        WHERE id = $1
        RETURNING *
        "#,
    )
    .bind(auth_user.user_id)
    .bind(&payload.first_name)
    .bind(&payload.last_name)
    .bind(&payload.middle_name)
    .bind(&payload.email)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(UserPublic::from(user)))
}

/// Загрузка аватара пользователя
#[utoipa::path(
    post,
    path = "/api/v1/users/me/avatar",
    tag = "users",
    security(("bearer_auth" = [])),
    request_body(content = String, content_type = "multipart/form-data"),
    responses(
        (status = 200, description = "Аватар загружен", body = AvatarUploadResponse),
        (status = 400, description = "Неверный формат файла"),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn upload_avatar(
    State(state): State<AppState>,
    auth_user: AuthUser,
    mut multipart: Multipart,
) -> AppResult<Json<Value>> {
    let file_service = FileService::new(&state.config).await?;

    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::BadRequest(e.to_string()))?
    {
        let name = field.name().unwrap_or("").to_string();

        if name == "avatar" {
            let content_type = field
                .content_type()
                .ok_or_else(|| AppError::BadRequest("Content-Type отсутствует".to_string()))?
                .to_string();

            if !validate_image_content_type(&content_type) {
                return Err(AppError::BadRequest(
                    "Недопустимый формат изображения".to_string(),
                ));
            }

            let file_name = field.file_name().unwrap_or("avatar.jpg").to_string();

            let data = field
                .bytes()
                .await
                .map_err(|e| AppError::BadRequest(e.to_string()))?;

            if data.len() > MAX_IMAGE_SIZE {
                return Err(AppError::BadRequest("Файл слишком большой".to_string()));
            }

            let url = file_service
                .upload_file("avatars", &file_name, &content_type, data.to_vec())
                .await?;

            // Обновляем аватар пользователя
            sqlx::query("UPDATE users SET avatar_url = $1, updated_at = NOW() WHERE id = $2")
                .bind(&url)
                .bind(auth_user.user_id)
                .execute(&state.pool)
                .await?;

            return Ok(Json(json!({
                "success": true,
                "avatar_url": url
            })));
        }
    }

    Err(AppError::BadRequest("Файл не найден".to_string()))
}

/// Получение списка квартир пользователя
#[utoipa::path(
    get,
    path = "/api/v1/users/me/apartments",
    tag = "users",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Список квартир", body = Vec<ApartmentResponse>),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn get_my_apartments(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Vec<ApartmentResponse>>> {
    let apartments = sqlx::query_as::<
        _,
        (
            uuid::Uuid,
            uuid::Uuid,
            Option<String>,
            Option<String>,
            String,
            Option<i32>,
            Option<rust_decimal::Decimal>,
            Option<i32>,
            bool,
            bool,
            bool,
            String,
        ),
    >(
        r#"
        SELECT
            a.id,
            a.complex_id,
            a.building,
            a.entrance,
            a.number,
            a.floor,
            a.area,
            a.rooms_count,
            a.owner_id = $1 as is_owner,
            a.resident_id = $1 as is_resident,
            a.is_ownership_verified,
            c.name as complex_name
        FROM apartments a
        JOIN complexes c ON c.id = a.complex_id
        WHERE a.owner_id = $1 OR a.resident_id = $1
        ORDER BY c.name, a.number
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.pool)
    .await?;

    let response: Vec<ApartmentResponse> = apartments
        .into_iter()
        .map(
            |(
                id,
                complex_id,
                building,
                entrance,
                number,
                floor,
                area,
                rooms_count,
                is_owner,
                is_resident,
                is_ownership_verified,
                complex_name,
            )| {
                ApartmentResponse {
                    id,
                    complex_id,
                    complex_name: Some(complex_name),
                    building,
                    entrance,
                    number,
                    floor,
                    area,
                    rooms_count,
                    is_owner,
                    is_resident,
                    is_ownership_verified,
                }
            },
        )
        .collect();

    Ok(Json(response))
}
