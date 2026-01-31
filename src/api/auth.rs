use axum::{extract::State, routing::post, Json, Router};
use chrono::{Duration, Utc};
use serde_json::{json, Value};

use crate::error::{AppError, AppResult};
use crate::middleware::AppState;
use crate::models::{
    AuthResponse, RefreshTokenRequest, SendCodeRequest, TokenResponse, UserPublic,
    VerifyCodeRequest,
};
use crate::services::{
    auth_service::{normalize_phone, validate_kz_phone},
    AuthService, SmsService,
};

/// Успешный ответ на отправку SMS-кода
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct SendCodeResponse {
    pub success: bool,
    pub message: String,
}

/// Успешный ответ на выход
#[derive(serde::Serialize, utoipa::ToSchema)]
pub struct LogoutResponse {
    pub success: bool,
    pub message: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/send-code", post(send_code))
        .route("/verify-code", post(verify_code))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
}

/// Отправка SMS-кода для входа
#[utoipa::path(
    post,
    path = "/api/v1/auth/send-code",
    tag = "auth",
    request_body = SendCodeRequest,
    responses(
        (status = 200, description = "Код успешно отправлен", body = SendCodeResponse),
        (status = 400, description = "Неверный формат номера телефона"),
        (status = 429, description = "Слишком много запросов")
    )
)]
pub async fn send_code(
    State(state): State<AppState>,
    Json(payload): Json<SendCodeRequest>,
) -> AppResult<Json<Value>> {
    let phone = normalize_phone(&payload.phone);

    if !validate_kz_phone(&phone) {
        return Err(AppError::Validation(
            "Неверный формат номера телефона".to_string(),
        ));
    }

    // Проверяем лимит отправки
    let recent_count: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM sms_codes
        WHERE phone = $1 AND created_at > NOW() - INTERVAL '1 hour'
        "#,
    )
    .bind(&phone)
    .fetch_one(&state.pool)
    .await?;

    if recent_count.0 >= 5 {
        return Err(AppError::TooManyRequests);
    }

    // Генерируем и сохраняем код
    let code = AuthService::generate_sms_code();
    AuthService::save_sms_code(&state.pool, &phone, &code).await?;

    // Отправляем SMS
    let sms_service = SmsService::new(state.config.clone());
    sms_service.send_code(&phone, &code).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Код отправлен"
    })))
}

/// Подтверждение SMS-кода и вход в систему
#[utoipa::path(
    post,
    path = "/api/v1/auth/verify-code",
    tag = "auth",
    request_body = VerifyCodeRequest,
    responses(
        (status = 200, description = "Успешный вход", body = AuthResponse),
        (status = 400, description = "Неверный код"),
        (status = 403, description = "Пользователь заблокирован"),
        (status = 429, description = "Слишком много попыток")
    )
)]
pub async fn verify_code(
    State(state): State<AppState>,
    Json(payload): Json<VerifyCodeRequest>,
) -> AppResult<Json<AuthResponse>> {
    let phone = normalize_phone(&payload.phone);

    // Проверяем код
    let is_valid = AuthService::verify_sms_code(&state.pool, &phone, &payload.code).await?;

    if !is_valid {
        // Проверяем количество попыток
        let attempts: Option<(i32,)> = sqlx::query_as(
            r#"
            SELECT attempts FROM sms_codes
            WHERE phone = $1 AND is_used = false AND expires_at > NOW()
            ORDER BY created_at DESC LIMIT 1
            "#,
        )
        .bind(&phone)
        .fetch_optional(&state.pool)
        .await?;

        if let Some((count,)) = attempts {
            if count >= 3 {
                return Err(AppError::TooManyAttempts);
            }
        }

        return Err(AppError::InvalidCode);
    }

    // Получаем или создаем пользователя
    let (user, is_new_user) = match AuthService::get_user_by_phone(&state.pool, &phone).await? {
        Some(user) => {
            if user.is_blocked {
                return Err(AppError::Forbidden);
            }
            (user, false)
        }
        None => {
            let user = AuthService::create_user(&state.pool, &phone).await?;
            (user, true)
        }
    };

    // Обновляем время последнего входа
    AuthService::update_last_login(&state.pool, user.id).await?;

    // Генерируем токены
    let auth_service = AuthService::new(state.config.clone());
    let access_token = auth_service.generate_access_token(&user)?;
    let refresh_token = auth_service.generate_refresh_token(&user)?;

    // Сохраняем refresh token
    let token_hash = AuthService::hash_token(&refresh_token);
    let expires_at = Utc::now() + Duration::seconds(state.config.jwt_refresh_expiry);

    AuthService::save_refresh_token(
        &state.pool,
        user.id,
        &token_hash,
        payload.device_info.as_deref(),
        None,
        expires_at,
    )
    .await?;

    Ok(Json(AuthResponse {
        access_token,
        refresh_token,
        user: UserPublic::from(user),
        is_new_user,
    }))
}

/// Обновление пары токенов
#[utoipa::path(
    post,
    path = "/api/v1/auth/refresh",
    tag = "auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Токены обновлены", body = TokenResponse),
        (status = 401, description = "Недействительный токен"),
        (status = 403, description = "Пользователь заблокирован")
    )
)]
pub async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> AppResult<Json<TokenResponse>> {
    let auth_service = AuthService::new(state.config.clone());

    // Проверяем refresh token
    let claims = auth_service.verify_token(&payload.refresh_token)?;

    if claims.token_type != "refresh" {
        return Err(AppError::Unauthorized);
    }

    let user_id = uuid::Uuid::parse_str(&claims.sub).map_err(|_| AppError::Unauthorized)?;

    // Проверяем, существует ли токен в базе
    let token_hash = AuthService::hash_token(&payload.refresh_token);
    let exists: Option<(i32,)> =
        sqlx::query_as("SELECT 1 FROM refresh_tokens WHERE token_hash = $1 AND expires_at > NOW()")
            .bind(&token_hash)
            .fetch_optional(&state.pool)
            .await?;

    if exists.is_none() {
        return Err(AppError::Unauthorized);
    }

    // Получаем пользователя
    let user = AuthService::get_user_by_id(&state.pool, user_id).await?;

    if user.is_blocked {
        return Err(AppError::Forbidden);
    }

    // Удаляем старый refresh token
    AuthService::delete_refresh_token(&state.pool, &token_hash).await?;

    // Генерируем новые токены
    let new_access_token = auth_service.generate_access_token(&user)?;
    let new_refresh_token = auth_service.generate_refresh_token(&user)?;

    // Сохраняем новый refresh token
    let new_token_hash = AuthService::hash_token(&new_refresh_token);
    let expires_at = Utc::now() + Duration::seconds(state.config.jwt_refresh_expiry);

    AuthService::save_refresh_token(
        &state.pool,
        user.id,
        &new_token_hash,
        None,
        None,
        expires_at,
    )
    .await?;

    Ok(Json(TokenResponse {
        access_token: new_access_token,
        refresh_token: new_refresh_token,
    }))
}

/// Выход из системы
#[utoipa::path(
    post,
    path = "/api/v1/auth/logout",
    tag = "auth",
    request_body = RefreshTokenRequest,
    responses(
        (status = 200, description = "Успешный выход", body = LogoutResponse)
    )
)]
pub async fn logout(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> AppResult<Json<Value>> {
    let token_hash = AuthService::hash_token(&payload.refresh_token);
    AuthService::delete_refresh_token(&state.pool, &token_hash).await?;

    Ok(Json(json!({
        "success": true,
        "message": "Выход выполнен"
    })))
}
