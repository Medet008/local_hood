use axum::{
    extract::{Path, Query, State},
    routing::{get, post, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{is_admin_or_higher, AppState, AuthUser};
use crate::models::{
    ChairmanApplication, ChairmanApplicationStatus, Complex, ComplexStatus, User, UserRole,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/dashboard", get(get_dashboard))
        .route("/complexes", get(list_complexes))
        .route("/complexes/:id/verify", put(verify_complex))
        .route("/users", get(list_users))
        .route("/users/:id/block", put(block_user))
        .route("/users/:id/role", put(change_role))
        .route("/chairman-applications", get(list_chairman_applications))
        .route("/chairman-applications/:id/approve", put(approve_chairman))
        .route("/chairman-applications/:id/reject", put(reject_chairman))
        .route("/logs", get(get_logs))
}

#[derive(Debug, Deserialize)]
struct PaginationQuery {
    page: Option<i64>,
    limit: Option<i64>,
    status: Option<String>,
    query: Option<String>,
}

fn check_admin(role: &UserRole) -> AppResult<()> {
    if !is_admin_or_higher(role) {
        return Err(AppError::Forbidden);
    }
    Ok(())
}

async fn get_dashboard(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Value>> {
    check_admin(&auth_user.role)?;

    let total_users: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
        .fetch_one(&state.pool)
        .await?;

    let total_complexes: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM complexes")
        .fetch_one(&state.pool)
        .await?;

    let active_complexes: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM complexes WHERE status = 'active'"
    )
    .fetch_one(&state.pool)
    .await?;

    let pending_complexes: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM complexes WHERE status = 'pending'"
    )
    .fetch_one(&state.pool)
    .await?;

    let total_apartments: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM apartments")
        .fetch_one(&state.pool)
        .await?;

    let pending_chairman_apps: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM chairman_applications WHERE status = 'pending'"
    )
    .fetch_one(&state.pool)
    .await?;

    let pending_join_requests: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM join_requests WHERE status = 'pending'"
    )
    .fetch_one(&state.pool)
    .await?;

    let new_users_today: (i64,) = sqlx::query_as(
        "SELECT COUNT(*) FROM users WHERE created_at::date = CURRENT_DATE"
    )
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(json!({
        "users": {
            "total": total_users.0,
            "new_today": new_users_today.0
        },
        "complexes": {
            "total": total_complexes.0,
            "active": active_complexes.0,
            "pending": pending_complexes.0
        },
        "apartments": {
            "total": total_apartments.0
        },
        "pending_actions": {
            "chairman_applications": pending_chairman_apps.0,
            "join_requests": pending_join_requests.0
        }
    })))
}

async fn list_complexes(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Value>>> {
    check_admin(&auth_user.role)?;

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.page.unwrap_or(0) * limit;
    let search = query.query.as_ref().map(|q| format!("%{}%", q));

    let complexes = sqlx::query_as::<_, Complex>(
        r#"
        SELECT * FROM complexes
        WHERE ($1::varchar IS NULL OR status::text = $1)
          AND ($2::varchar IS NULL OR name ILIKE $2)
        ORDER BY created_at DESC
        LIMIT $3 OFFSET $4
        "#
    )
    .bind(&query.status)
    .bind(&search)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let response: Vec<Value> = complexes.into_iter().map(|c| {
        json!({
            "id": c.id,
            "name": c.name,
            "city_id": c.city_id,
            "status": format!("{:?}", c.status).to_lowercase(),
            "apartments_count": c.apartments_count,
            "created_at": c.created_at
        })
    }).collect();

    Ok(Json(response))
}

async fn verify_complex(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    check_admin(&auth_user.role)?;

    sqlx::query(
        r#"
        UPDATE complexes
        SET status = 'active', verified_at = NOW(), verified_by = $2
        WHERE id = $1
        "#
    )
    .bind(id)
    .bind(auth_user.user_id)
    .execute(&state.pool)
    .await?;

    // Создаем ОСИ для ЖК
    sqlx::query(
        r#"
        INSERT INTO osi (complex_id, name)
        SELECT $1, name || ' ОСИ' FROM complexes WHERE id = $1
        ON CONFLICT (complex_id) DO NOTHING
        "#
    )
    .bind(id)
    .execute(&state.pool)
    .await?;

    // Создаем общий чат ЖК
    sqlx::query(
        r#"
        INSERT INTO chats (complex_id, chat_type, name)
        SELECT $1, 'complex', 'Общий чат ' || name FROM complexes WHERE id = $1
        "#
    )
    .bind(id)
    .execute(&state.pool)
    .await?;

    // Логируем
    log_admin_action(&state, auth_user.user_id, "verify_complex", "complex", id).await?;

    Ok(Json(json!({"success": true})))
}

async fn list_users(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Value>>> {
    check_admin(&auth_user.role)?;

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.page.unwrap_or(0) * limit;
    let search = query.query.as_ref().map(|q| format!("%{}%", q));

    let users = sqlx::query_as::<_, User>(
        r#"
        SELECT * FROM users
        WHERE ($1::varchar IS NULL OR phone ILIKE $1 OR first_name ILIKE $1 OR last_name ILIKE $1)
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(&search)
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let response: Vec<Value> = users.into_iter().map(|u| {
        json!({
            "id": u.id,
            "phone": u.phone,
            "first_name": u.first_name,
            "last_name": u.last_name,
            "role": format!("{:?}", u.role).to_lowercase(),
            "is_verified": u.is_verified,
            "is_blocked": u.is_blocked,
            "created_at": u.created_at
        })
    }).collect();

    Ok(Json(response))
}

async fn block_user(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<Value>,
) -> AppResult<Json<Value>> {
    check_admin(&auth_user.role)?;

    let block = payload["block"].as_bool().unwrap_or(true);
    let reason = payload["reason"].as_str();

    if block {
        sqlx::query(
            "UPDATE users SET is_blocked = true, blocked_reason = $2, blocked_at = NOW() WHERE id = $1"
        )
        .bind(id)
        .bind(reason)
        .execute(&state.pool)
        .await?;
    } else {
        sqlx::query(
            "UPDATE users SET is_blocked = false, blocked_reason = NULL, blocked_at = NULL WHERE id = $1"
        )
        .bind(id)
        .execute(&state.pool)
        .await?;
    }

    log_admin_action(&state, auth_user.user_id, if block { "block_user" } else { "unblock_user" }, "user", id).await?;

    Ok(Json(json!({"success": true})))
}

async fn change_role(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<Value>,
) -> AppResult<Json<Value>> {
    check_admin(&auth_user.role)?;

    let role_str = payload["role"].as_str()
        .ok_or_else(|| AppError::BadRequest("role обязателен".to_string()))?;

    let role = match role_str {
        "user" => UserRole::User,
        "resident" => UserRole::Resident,
        "owner" => UserRole::Owner,
        "council" => UserRole::Council,
        "chairman" => UserRole::Chairman,
        "moderator" => UserRole::Moderator,
        "admin" => {
            // Только SuperAdmin может назначать админов
            if auth_user.role != UserRole::SuperAdmin {
                return Err(AppError::Forbidden);
            }
            UserRole::Admin
        }
        _ => return Err(AppError::BadRequest("Неверная роль".to_string())),
    };

    sqlx::query("UPDATE users SET role = $2, updated_at = NOW() WHERE id = $1")
        .bind(id)
        .bind(role)
        .execute(&state.pool)
        .await?;

    log_admin_action(&state, auth_user.user_id, "change_role", "user", id).await?;

    Ok(Json(json!({"success": true})))
}

async fn list_chairman_applications(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Value>>> {
    check_admin(&auth_user.role)?;

    let limit = query.limit.unwrap_or(50).min(100);
    let offset = query.page.unwrap_or(0) * limit;

    let applications = sqlx::query_as::<_, ChairmanApplication>(
        r#"
        SELECT * FROM chairman_applications
        WHERE ($1::varchar IS NULL OR status::text = $1)
        ORDER BY created_at DESC
        LIMIT $2 OFFSET $3
        "#
    )
    .bind(query.status.as_deref().unwrap_or("pending"))
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for app in applications {
        let user: (String, String) = sqlx::query_as(
            "SELECT COALESCE(first_name || ' ' || last_name, phone), phone FROM users WHERE id = $1"
        )
        .bind(app.user_id)
        .fetch_one(&state.pool)
        .await?;

        let complex: (String,) = sqlx::query_as(
            "SELECT name FROM complexes WHERE id = $1"
        )
        .bind(app.complex_id)
        .fetch_one(&state.pool)
        .await?;

        response.push(json!({
            "id": app.id,
            "user_id": app.user_id,
            "user_name": user.0,
            "user_phone": user.1,
            "complex_id": app.complex_id,
            "complex_name": complex.0,
            "motivation": app.motivation,
            "document_url": app.document_url,
            "status": format!("{:?}", app.status).to_lowercase(),
            "created_at": app.created_at
        }));
    }

    Ok(Json(response))
}

async fn approve_chairman(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    check_admin(&auth_user.role)?;

    let app = sqlx::query_as::<_, ChairmanApplication>(
        "SELECT * FROM chairman_applications WHERE id = $1 AND status = 'pending'"
    )
    .bind(id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Заявка не найдена".to_string()))?;

    // Обновляем заявку
    sqlx::query(
        r#"
        UPDATE chairman_applications
        SET status = 'approved', reviewed_by = $2, reviewed_at = NOW()
        WHERE id = $1
        "#
    )
    .bind(id)
    .bind(auth_user.user_id)
    .execute(&state.pool)
    .await?;

    // Назначаем председателем
    sqlx::query(
        "UPDATE osi SET chairman_id = $2 WHERE complex_id = $1"
    )
    .bind(app.complex_id)
    .bind(app.user_id)
    .execute(&state.pool)
    .await?;

    // Обновляем роль пользователя
    sqlx::query("UPDATE users SET role = 'chairman' WHERE id = $1")
        .bind(app.user_id)
        .execute(&state.pool)
        .await?;

    log_admin_action(&state, auth_user.user_id, "approve_chairman", "chairman_application", id).await?;

    Ok(Json(json!({"success": true})))
}

async fn reject_chairman(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(id): Path<Uuid>,
    Json(payload): Json<Value>,
) -> AppResult<Json<Value>> {
    check_admin(&auth_user.role)?;

    let reason = payload["reason"].as_str();

    sqlx::query(
        r#"
        UPDATE chairman_applications
        SET status = 'rejected', reviewed_by = $2, reviewed_at = NOW(), rejection_reason = $3
        WHERE id = $1 AND status = 'pending'
        "#
    )
    .bind(id)
    .bind(auth_user.user_id)
    .bind(reason)
    .execute(&state.pool)
    .await?;

    log_admin_action(&state, auth_user.user_id, "reject_chairman", "chairman_application", id).await?;

    Ok(Json(json!({"success": true})))
}

async fn get_logs(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Query(query): Query<PaginationQuery>,
) -> AppResult<Json<Vec<Value>>> {
    check_admin(&auth_user.role)?;

    let limit = query.limit.unwrap_or(100).min(500);
    let offset = query.page.unwrap_or(0) * limit;

    let logs = sqlx::query_as::<_, (Uuid, Option<Uuid>, String, Option<String>, Option<Uuid>, chrono::DateTime<chrono::Utc>)>(
        r#"
        SELECT id, user_id, action, entity_type, entity_id, created_at
        FROM admin_logs
        ORDER BY created_at DESC
        LIMIT $1 OFFSET $2
        "#
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for (id, user_id, action, entity_type, entity_id, created_at) in logs {
        let user_name = if let Some(uid) = user_id {
            sqlx::query_as::<_, (String,)>(
                "SELECT COALESCE(first_name || ' ' || last_name, phone) FROM users WHERE id = $1"
            )
            .bind(uid)
            .fetch_optional(&state.pool)
            .await?
            .map(|(n,)| n)
        } else {
            None
        };

        response.push(json!({
            "id": id,
            "user_name": user_name,
            "action": action,
            "entity_type": entity_type,
            "entity_id": entity_id,
            "created_at": created_at
        }));
    }

    Ok(Json(response))
}

async fn log_admin_action(
    state: &AppState,
    user_id: Uuid,
    action: &str,
    entity_type: &str,
    entity_id: Uuid,
) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO admin_logs (user_id, action, entity_type, entity_id) VALUES ($1, $2, $3, $4)"
    )
    .bind(user_id)
    .bind(action)
    .bind(entity_type)
    .bind(entity_id)
    .execute(&state.pool)
    .await?;

    Ok(())
}
