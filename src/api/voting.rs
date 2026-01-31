use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use rust_decimal::Decimal;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{is_chairman_or_higher, is_owner_or_higher, AppState, AuthUser};
use crate::models::{
    CastVoteRequest, CreateVotingRequest, Voting, VotingOption, VotingOptionResponse,
    VotingResponse, VotingStatus, VotingType,
};

/// Успешный ответ
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SuccessResponse {
    pub success: bool,
}

/// Ответ на голосование
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct VoteResponse {
    pub success: bool,
    pub message: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_votings))
        .route("/", post(create_voting))
        .route("/:id", get(get_voting))
        .route("/:id/vote", post(cast_vote))
        .route("/:id/close", post(close_voting))
}

#[derive(Debug, Deserialize, utoipa::ToSchema, utoipa::IntoParams)]
pub struct VotingsQuery {
    pub status: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
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

/// Получить список голосований
#[utoipa::path(
    get,
    path = "/api/v1/voting",
    tag = "voting",
    security(("bearer_auth" = [])),
    params(
        ("status" = Option<String>, Query, description = "Статус (draft, active, closed)"),
        ("page" = Option<i64>, Query, description = "Номер страницы"),
        ("limit" = Option<i64>, Query, description = "Количество записей")
    ),
    responses(
        (status = 200, description = "Список голосований", body = Vec<VotingResponse>),
        (status = 401, description = "Не авторизован")
    )
)]
pub async fn list_votings(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<VotingsQuery>,
) -> AppResult<Json<Vec<VotingResponse>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    let limit = query.limit.unwrap_or(20).min(100);
    let offset = query.page.unwrap_or(0) * limit;

    let votings = sqlx::query_as::<_, Voting>(
        r#"
        SELECT * FROM votings
        WHERE complex_id = $1
          AND ($2::varchar IS NULL OR status::text = $2)
        ORDER BY starts_at DESC
        LIMIT $3 OFFSET $4
        "#,
    )
    .bind(complex_id)
    .bind(&query.status)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for voting in votings {
        response.push(build_voting_response(&state, &voting, auth_user.user_id).await?);
    }

    Ok(Json(response))
}

async fn build_voting_response(
    state: &AppState,
    voting: &Voting,
    user_id: Uuid,
) -> AppResult<VotingResponse> {
    let options = sqlx::query_as::<_, VotingOption>(
        "SELECT * FROM voting_options WHERE voting_id = $1 ORDER BY sort_order",
    )
    .bind(voting.id)
    .fetch_all(&state.pool)
    .await?;

    let total_votes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM votes WHERE voting_id = $1")
        .bind(voting.id)
        .fetch_one(&state.pool)
        .await?;

    let total_weight: (Decimal,) =
        sqlx::query_as("SELECT COALESCE(SUM(vote_weight), 0) FROM votes WHERE voting_id = $1")
            .bind(voting.id)
            .fetch_one(&state.pool)
            .await?;

    let user_voted: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM votes WHERE voting_id = $1 AND user_id = $2")
            .bind(voting.id)
            .bind(user_id)
            .fetch_optional(&state.pool)
            .await?;

    let mut option_responses = Vec::new();
    for opt in options {
        let votes_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM votes WHERE option_id = $1")
            .bind(opt.id)
            .fetch_one(&state.pool)
            .await?;

        let votes_weight: (Decimal,) =
            sqlx::query_as("SELECT COALESCE(SUM(vote_weight), 0) FROM votes WHERE option_id = $1")
                .bind(opt.id)
                .fetch_one(&state.pool)
                .await?;

        let percentage = if total_weight.0 > Decimal::ZERO {
            (votes_weight.0 / total_weight.0 * Decimal::from(100))
                .to_string()
                .parse::<f64>()
                .unwrap_or(0.0)
        } else {
            0.0
        };

        option_responses.push(VotingOptionResponse {
            id: opt.id,
            text: opt.text,
            votes_count: votes_count.0 as i32,
            votes_weight: votes_weight.0,
            percentage,
        });
    }

    Ok(VotingResponse {
        id: voting.id,
        title: voting.title.clone(),
        description: voting.description.clone(),
        voting_type: voting.voting_type.clone(),
        status: voting.status.clone(),
        requires_owner: voting.requires_owner,
        quorum_percent: voting.quorum_percent,
        starts_at: voting.starts_at,
        ends_at: voting.ends_at,
        options: option_responses,
        total_votes: total_votes.0 as i32,
        total_weight: total_weight.0,
        user_voted: user_voted.is_some(),
        created_at: voting.created_at,
    })
}

/// Получить голосование по ID
#[utoipa::path(
    get,
    path = "/api/v1/voting/{id}",
    tag = "voting",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID голосования")
    ),
    responses(
        (status = 200, description = "Голосование", body = VotingResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Не найдено")
    )
)]
pub async fn get_voting(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<VotingResponse>> {
    let voting = sqlx::query_as::<_, Voting>("SELECT * FROM votings WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Голосование не найдено".to_string()))?;

    let response = build_voting_response(&state, &voting, auth_user.user_id).await?;
    Ok(Json(response))
}

/// Создать голосование
#[utoipa::path(
    post,
    path = "/api/v1/voting",
    tag = "voting",
    security(("bearer_auth" = [])),
    request_body = CreateVotingRequest,
    responses(
        (status = 200, description = "Голосование создано", body = VotingResponse),
        (status = 400, description = "Минимум 2 варианта ответа"),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет прав")
    )
)]
pub async fn create_voting(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreateVotingRequest>,
) -> AppResult<Json<VotingResponse>> {
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

    if payload.options.len() < 2 {
        return Err(AppError::BadRequest(
            "Минимум 2 варианта ответа".to_string(),
        ));
    }

    let voting = sqlx::query_as::<_, Voting>(
        r#"
        INSERT INTO votings (
            complex_id, title, description, voting_type, status,
            requires_owner, quorum_percent, starts_at, ends_at, created_by
        )
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        RETURNING *
        "#,
    )
    .bind(complex_id)
    .bind(&payload.title)
    .bind(&payload.description)
    .bind(
        payload
            .voting_type
            .clone()
            .unwrap_or(VotingType::SingleChoice),
    )
    .bind(VotingStatus::Draft)
    .bind(payload.requires_owner.unwrap_or(true))
    .bind(payload.quorum_percent.unwrap_or(51))
    .bind(payload.starts_at)
    .bind(payload.ends_at)
    .bind(auth_user.user_id)
    .fetch_one(&state.pool)
    .await?;

    for (i, option_text) in payload.options.iter().enumerate() {
        sqlx::query("INSERT INTO voting_options (voting_id, text, sort_order) VALUES ($1, $2, $3)")
            .bind(voting.id)
            .bind(option_text)
            .bind(i as i32)
            .execute(&state.pool)
            .await?;
    }

    let response = build_voting_response(&state, &voting, auth_user.user_id).await?;
    Ok(Json(response))
}

/// Проголосовать
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/vote",
    tag = "voting",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID голосования")
    ),
    request_body = CastVoteRequest,
    responses(
        (status = 200, description = "Голос принят", body = VoteResponse),
        (status = 400, description = "Голосование не активно"),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет прав голосовать"),
        (status = 404, description = "Голосование не найдено"),
        (status = 409, description = "Вы уже голосовали")
    )
)]
pub async fn cast_vote(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<CastVoteRequest>,
) -> AppResult<Json<Value>> {
    let voting = sqlx::query_as::<_, Voting>("SELECT * FROM votings WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Голосование не найдено".to_string()))?;

    if voting.status != VotingStatus::Active {
        return Err(AppError::BadRequest("Голосование не активно".to_string()));
    }

    let now = chrono::Utc::now();
    if now < voting.starts_at || now > voting.ends_at {
        return Err(AppError::BadRequest(
            "Голосование не в активном периоде".to_string(),
        ));
    }

    if voting.requires_owner && !is_owner_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    let existing_vote: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM votes WHERE voting_id = $1 AND user_id = $2")
            .bind(id)
            .bind(auth_user.user_id)
            .fetch_optional(&state.pool)
            .await?;

    if existing_vote.is_some() {
        return Err(AppError::Conflict("Вы уже голосовали".to_string()));
    }

    let option_exists: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM voting_options WHERE id = $1 AND voting_id = $2")
            .bind(payload.option_id)
            .bind(id)
            .fetch_optional(&state.pool)
            .await?;

    if option_exists.is_none() {
        return Err(AppError::BadRequest("Неверный вариант ответа".to_string()));
    }

    let vote_weight: (Decimal,) = sqlx::query_as(
        r#"
        SELECT COALESCE(SUM(area), 1)
        FROM apartments
        WHERE complex_id = $1 AND owner_id = $2
        "#,
    )
    .bind(voting.complex_id)
    .bind(auth_user.user_id)
    .fetch_one(&state.pool)
    .await?;

    let apartment_id: Option<(Uuid,)> =
        sqlx::query_as("SELECT id FROM apartments WHERE complex_id = $1 AND owner_id = $2 LIMIT 1")
            .bind(voting.complex_id)
            .bind(auth_user.user_id)
            .fetch_optional(&state.pool)
            .await?;

    sqlx::query(
        r#"
        INSERT INTO votes (voting_id, option_id, user_id, apartment_id, vote_weight)
        VALUES ($1, $2, $3, $4, $5)
        "#,
    )
    .bind(id)
    .bind(payload.option_id)
    .bind(auth_user.user_id)
    .bind(apartment_id.map(|(id,)| id))
    .bind(vote_weight.0)
    .execute(&state.pool)
    .await?;

    Ok(Json(json!({
        "success": true,
        "message": "Голос принят"
    })))
}

/// Закрыть голосование
#[utoipa::path(
    post,
    path = "/api/v1/voting/{id}/close",
    tag = "voting",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID голосования")
    ),
    responses(
        (status = 200, description = "Голосование закрыто", body = SuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет прав"),
        (status = 404, description = "Не найдено")
    )
)]
pub async fn close_voting(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let voting = sqlx::query_as::<_, Voting>("SELECT * FROM votings WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Голосование не найдено".to_string()))?;

    if voting.created_by != auth_user.user_id && !is_chairman_or_higher(&auth_user.role) {
        return Err(AppError::Forbidden);
    }

    sqlx::query("UPDATE votings SET status = 'closed', updated_at = NOW() WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    Ok(Json(json!({"success": true})))
}
