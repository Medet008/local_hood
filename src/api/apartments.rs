use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{is_chairman_or_higher, AppState, AuthUser};
use crate::models::{
    JoinRequest, JoinRequestResponse, JoinRequestStatus, ReviewJoinRequestRequest, UserRole,
};

/// Ответ на рассмотрение заявки
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct ReviewResponse {
    pub success: bool,
    pub message: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/join-requests", get(get_join_requests))
        .route("/join-requests/:id", put(review_join_request))
}

/// Получение заявок на присоединение
#[utoipa::path(
    get,
    path = "/api/v1/apartments/join-requests",
    tag = "apartments",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Список заявок", body = Vec<JoinRequestResponse>),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет прав")
    )
)]
pub async fn get_join_requests(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Vec<JoinRequestResponse>>> {
    // Получаем ЖК, где пользователь председатель
    let complex_ids: Vec<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT complex_id FROM osi WHERE chairman_id = $1
        UNION
        SELECT id FROM complexes WHERE created_by = $1
        "#,
    )
    .bind(auth_user.user_id)
    .fetch_all(&state.pool)
    .await?;

    if complex_ids.is_empty() && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    let ids: Vec<Uuid> = complex_ids.into_iter().map(|(id,)| id).collect();

    let requests = sqlx::query_as::<_, JoinRequest>(
        r#"
        SELECT * FROM join_requests
        WHERE complex_id = ANY($1) AND status = 'pending'
        ORDER BY created_at DESC
        "#,
    )
    .bind(&ids)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for req in requests {
        let user_info: Option<(String, String)> = sqlx::query_as(
            "SELECT COALESCE(first_name || ' ' || last_name, phone), phone FROM users WHERE id = $1"
        )
        .bind(req.user_id)
        .fetch_optional(&state.pool)
        .await?;

        let (user_name, user_phone) = user_info.unwrap_or(("".to_string(), "".to_string()));

        response.push(JoinRequestResponse {
            id: req.id,
            user_id: req.user_id,
            user_name: Some(user_name),
            user_phone: Some(user_phone),
            complex_id: req.complex_id,
            apartment_number: req.apartment_number,
            building: req.building,
            is_owner: req.is_owner,
            document_url: req.document_url,
            status: req.status,
            created_at: req.created_at,
        });
    }

    Ok(Json(response))
}

/// Рассмотрение заявки на присоединение
#[utoipa::path(
    put,
    path = "/api/v1/apartments/join-requests/{id}",
    tag = "apartments",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID заявки")
    ),
    request_body = ReviewJoinRequestRequest,
    responses(
        (status = 200, description = "Заявка рассмотрена", body = ReviewResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет прав"),
        (status = 404, description = "Заявка не найдена")
    )
)]
pub async fn review_join_request(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(request_id): Path<Uuid>,
    Json(payload): Json<ReviewJoinRequestRequest>,
) -> AppResult<Json<Value>> {
    // Получаем заявку
    let request = sqlx::query_as::<_, JoinRequest>(
        "SELECT * FROM join_requests WHERE id = $1 AND status = 'pending'",
    )
    .bind(request_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Заявка не найдена".to_string()))?;

    // Проверяем права
    let is_chairman: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM osi WHERE complex_id = $1 AND chairman_id = $2")
            .bind(request.complex_id)
            .bind(auth_user.user_id)
            .fetch_optional(&state.pool)
            .await?;

    if is_chairman.is_none() && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    if payload.approved {
        // Создаём или находим квартиру
        let apartment_id: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO apartments (complex_id, building, number)
            VALUES ($1, $2, $3)
            ON CONFLICT (complex_id, building, number) DO UPDATE SET updated_at = NOW()
            RETURNING id
            "#,
        )
        .bind(request.complex_id)
        .bind(&request.building)
        .bind(&request.apartment_number)
        .fetch_one(&state.pool)
        .await?;

        // Привязываем пользователя
        if request.is_owner {
            sqlx::query("UPDATE apartments SET owner_id = $1, updated_at = NOW() WHERE id = $2")
                .bind(request.user_id)
                .bind(apartment_id.0)
                .execute(&state.pool)
                .await?;

            // Повышаем роль до Owner
            sqlx::query(
                "UPDATE users SET role = $1 WHERE id = $2 AND role IN ('user', 'resident')",
            )
            .bind(UserRole::Owner)
            .bind(request.user_id)
            .execute(&state.pool)
            .await?;
        } else {
            sqlx::query("UPDATE apartments SET resident_id = $1, updated_at = NOW() WHERE id = $2")
                .bind(request.user_id)
                .bind(apartment_id.0)
                .execute(&state.pool)
                .await?;

            // Повышаем роль до Resident
            sqlx::query("UPDATE users SET role = $1 WHERE id = $2 AND role = 'user'")
                .bind(UserRole::Resident)
                .bind(request.user_id)
                .execute(&state.pool)
                .await?;
        }

        // Обновляем заявку
        sqlx::query(
            r#"
            UPDATE join_requests
            SET status = 'approved', apartment_id = $1, reviewed_by = $2, reviewed_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(apartment_id.0)
        .bind(auth_user.user_id)
        .bind(request_id)
        .execute(&state.pool)
        .await?;

        Ok(Json(json!({
            "success": true,
            "message": "Заявка одобрена"
        })))
    } else {
        // Отклоняем заявку
        sqlx::query(
            r#"
            UPDATE join_requests
            SET status = 'rejected', rejection_reason = $1, reviewed_by = $2, reviewed_at = NOW()
            WHERE id = $3
            "#,
        )
        .bind(&payload.rejection_reason)
        .bind(auth_user.user_id)
        .bind(request_id)
        .execute(&state.pool)
        .await?;

        Ok(Json(json!({
            "success": true,
            "message": "Заявка отклонена"
        })))
    }
}
