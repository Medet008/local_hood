use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use utoipa::ToSchema;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::middleware::{AppState, AuthUser};
use crate::models::{
    Chat, ChatMessage, ChatMessageResponse, ChatResponse, ChatType, CreatePrivateChatRequest,
    MessagePreview, MessagesQuery, SendChatMessageRequest, SenderInfo,
};

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ChatSuccessResponse {
    pub success: bool,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_chats))
        .route("/private", post(create_private_chat))
        .route("/:id/messages", get(get_messages))
        .route("/:id/messages", post(send_message))
        .route("/:id/read", post(mark_chat_as_read))
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

/// Получить список чатов пользователя
#[utoipa::path(
    get,
    path = "/api/chats",
    tag = "Чаты",
    security(("bearer_auth" = [])),
    responses(
        (status = 200, description = "Список чатов", body = Vec<ChatResponse>),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа")
    )
)]
async fn list_chats(
    State(state): State<AppState>,
    auth_user: AuthUser,
) -> AppResult<Json<Vec<ChatResponse>>> {
    let complex_id = get_user_complex(&state, auth_user.user_id).await?;

    // Получаем чаты пользователя
    let chats = sqlx::query_as::<_, Chat>(
        r#"
        SELECT c.* FROM chats c
        LEFT JOIN chat_members cm ON cm.chat_id = c.id
        WHERE (c.complex_id = $1 AND c.chat_type IN ('complex', 'building'))
           OR cm.user_id = $2
        GROUP BY c.id
        ORDER BY c.updated_at DESC
        "#,
    )
    .bind(complex_id)
    .bind(auth_user.user_id)
    .fetch_all(&state.pool)
    .await?;

    let mut response = Vec::new();
    for chat in chats {
        let last_message: Option<(String, Uuid, chrono::DateTime<chrono::Utc>)> = sqlx::query_as(
            r#"
            SELECT content, sender_id, created_at
            FROM chat_messages
            WHERE chat_id = $1 AND is_deleted = false
            ORDER BY created_at DESC
            LIMIT 1
            "#,
        )
        .bind(chat.id)
        .fetch_optional(&state.pool)
        .await?;

        let last_message_preview = if let Some((content, sender_id, created_at)) = last_message {
            let sender_name: (String,) =
                sqlx::query_as("SELECT COALESCE(first_name, phone) FROM users WHERE id = $1")
                    .bind(sender_id)
                    .fetch_one(&state.pool)
                    .await?;

            Some(MessagePreview {
                content,
                sender_name: sender_name.0,
                created_at,
            })
        } else {
            None
        };

        let unread_count: (i64,) = sqlx::query_as(
            r#"
            SELECT COUNT(*) FROM chat_messages m
            LEFT JOIN message_reads r ON r.message_id = m.id AND r.user_id = $2
            WHERE m.chat_id = $1 AND r.id IS NULL AND m.sender_id != $2
            "#,
        )
        .bind(chat.id)
        .bind(auth_user.user_id)
        .fetch_one(&state.pool)
        .await?;

        let members_count: (i64,) =
            sqlx::query_as("SELECT COUNT(*) FROM chat_members WHERE chat_id = $1")
                .bind(chat.id)
                .fetch_one(&state.pool)
                .await?;

        response.push(ChatResponse {
            id: chat.id,
            chat_type: chat.chat_type,
            name: chat.name,
            last_message: last_message_preview,
            unread_count: unread_count.0 as i32,
            members_count: members_count.0 as i32,
        });
    }

    Ok(Json(response))
}

/// Создать приватный чат с пользователем
#[utoipa::path(
    post,
    path = "/api/chats/private",
    tag = "Чаты",
    security(("bearer_auth" = [])),
    request_body = CreatePrivateChatRequest,
    responses(
        (status = 200, description = "Чат создан", body = ChatResponse),
        (status = 401, description = "Не авторизован"),
        (status = 404, description = "Пользователь не найден")
    )
)]
async fn create_private_chat(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Json(payload): Json<CreatePrivateChatRequest>,
) -> AppResult<Json<ChatResponse>> {
    // Проверяем, что пользователь существует
    let user_exists: Option<(i32,)> = sqlx::query_as("SELECT 1 FROM users WHERE id = $1")
        .bind(payload.user_id)
        .fetch_optional(&state.pool)
        .await?;

    if user_exists.is_none() {
        return Err(AppError::NotFound("Пользователь не найден".to_string()));
    }

    // Проверяем, нет ли уже чата
    let existing_chat: Option<(Uuid,)> = sqlx::query_as(
        r#"
        SELECT c.id FROM chats c
        JOIN chat_members cm1 ON cm1.chat_id = c.id AND cm1.user_id = $1
        JOIN chat_members cm2 ON cm2.chat_id = c.id AND cm2.user_id = $2
        WHERE c.chat_type = 'private'
        "#,
    )
    .bind(auth_user.user_id)
    .bind(payload.user_id)
    .fetch_optional(&state.pool)
    .await?;

    let chat_id = if let Some((id,)) = existing_chat {
        id
    } else {
        // Создаем новый чат
        let chat: (Uuid,) = sqlx::query_as(
            r#"
            INSERT INTO chats (chat_type, is_private, created_by)
            VALUES ('private', true, $1)
            RETURNING id
            "#,
        )
        .bind(auth_user.user_id)
        .fetch_one(&state.pool)
        .await?;

        // Добавляем участников
        sqlx::query("INSERT INTO chat_members (chat_id, user_id) VALUES ($1, $2), ($1, $3)")
            .bind(chat.0)
            .bind(auth_user.user_id)
            .bind(payload.user_id)
            .execute(&state.pool)
            .await?;

        chat.0
    };

    let chat = sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = $1")
        .bind(chat_id)
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(ChatResponse {
        id: chat.id,
        chat_type: chat.chat_type,
        name: chat.name,
        last_message: None,
        unread_count: 0,
        members_count: 2,
    }))
}

/// Получить сообщения чата
#[utoipa::path(
    get,
    path = "/api/chats/{id}/messages",
    tag = "Чаты",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID чата"),
        ("limit" = Option<i64>, Query, description = "Лимит сообщений"),
        ("before" = Option<Uuid>, Query, description = "Получить сообщения до указанного ID")
    ),
    responses(
        (status = 200, description = "Список сообщений", body = Vec<ChatMessageResponse>),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа к чату"),
        (status = 404, description = "Чат не найден")
    )
)]
async fn get_messages(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(chat_id): Path<Uuid>,
    Query(query): Query<MessagesQuery>,
) -> AppResult<Json<Vec<ChatMessageResponse>>> {
    // Проверяем доступ к чату
    let has_access = check_chat_access(&state, chat_id, auth_user.user_id).await?;
    if !has_access {
        return Err(AppError::Forbidden);
    }

    let limit = query.limit.unwrap_or(50).min(100);

    let messages = if let Some(before_id) = query.before {
        sqlx::query_as::<_, ChatMessage>(
            r#"
            SELECT * FROM chat_messages
            WHERE chat_id = $1 AND is_deleted = false AND id < $2
            ORDER BY created_at DESC
            LIMIT $3
            "#,
        )
        .bind(chat_id)
        .bind(before_id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    } else {
        sqlx::query_as::<_, ChatMessage>(
            r#"
            SELECT * FROM chat_messages
            WHERE chat_id = $1 AND is_deleted = false
            ORDER BY created_at DESC
            LIMIT $2
            "#,
        )
        .bind(chat_id)
        .bind(limit)
        .fetch_all(&state.pool)
        .await?
    };

    let mut response = Vec::new();
    for msg in messages {
        let sender: (Uuid, Option<String>, Option<String>) = sqlx::query_as(
            "SELECT id, COALESCE(first_name, phone), avatar_url FROM users WHERE id = $1",
        )
        .bind(msg.sender_id)
        .fetch_one(&state.pool)
        .await?;

        response.push(ChatMessageResponse {
            id: msg.id,
            sender: SenderInfo {
                id: sender.0,
                name: sender.1.unwrap_or_default(),
                avatar_url: sender.2,
            },
            content: if msg.is_deleted {
                "Сообщение удалено".to_string()
            } else {
                msg.content
            },
            attachment_url: msg.attachment_url,
            attachment_type: msg.attachment_type,
            reply_to: None, // Упрощено
            is_edited: msg.is_edited,
            is_deleted: msg.is_deleted,
            created_at: msg.created_at,
        });
    }

    // Помечаем сообщения как прочитанные
    mark_messages_as_read(&state, chat_id, auth_user.user_id).await?;

    Ok(Json(response))
}

/// Отправить сообщение в чат
#[utoipa::path(
    post,
    path = "/api/chats/{id}/messages",
    tag = "Чаты",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID чата")
    ),
    request_body = SendChatMessageRequest,
    responses(
        (status = 200, description = "Сообщение отправлено", body = ChatMessageResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа к чату"),
        (status = 404, description = "Чат не найден")
    )
)]
async fn send_message(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(chat_id): Path<Uuid>,
    Json(payload): Json<SendChatMessageRequest>,
) -> AppResult<Json<ChatMessageResponse>> {
    let has_access = check_chat_access(&state, chat_id, auth_user.user_id).await?;
    if !has_access {
        return Err(AppError::Forbidden);
    }

    let message = sqlx::query_as::<_, ChatMessage>(
        r#"
        INSERT INTO chat_messages (chat_id, sender_id, content, attachment_url, attachment_type, reply_to_id)
        VALUES ($1, $2, $3, $4, $5, $6)
        RETURNING *
        "#
    )
    .bind(chat_id)
    .bind(auth_user.user_id)
    .bind(&payload.content)
    .bind(&payload.attachment_url)
    .bind(&payload.attachment_type)
    .bind(&payload.reply_to_id)
    .fetch_one(&state.pool)
    .await?;

    // Обновляем время последнего сообщения в чате
    sqlx::query("UPDATE chats SET updated_at = NOW() WHERE id = $1")
        .bind(chat_id)
        .execute(&state.pool)
        .await?;

    let sender: (Uuid, Option<String>, Option<String>) = sqlx::query_as(
        "SELECT id, COALESCE(first_name, phone), avatar_url FROM users WHERE id = $1",
    )
    .bind(auth_user.user_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(ChatMessageResponse {
        id: message.id,
        sender: SenderInfo {
            id: sender.0,
            name: sender.1.unwrap_or_default(),
            avatar_url: sender.2,
        },
        content: message.content,
        attachment_url: message.attachment_url,
        attachment_type: message.attachment_type,
        reply_to: None,
        is_edited: false,
        is_deleted: false,
        created_at: message.created_at,
    }))
}

/// Отметить чат как прочитанный
#[utoipa::path(
    post,
    path = "/api/chats/{id}/read",
    tag = "Чаты",
    security(("bearer_auth" = [])),
    params(
        ("id" = Uuid, Path, description = "ID чата")
    ),
    responses(
        (status = 200, description = "Чат отмечен как прочитанный", body = ChatSuccessResponse),
        (status = 401, description = "Не авторизован"),
        (status = 403, description = "Нет доступа к чату"),
        (status = 404, description = "Чат не найден")
    )
)]
async fn mark_chat_as_read(
    State(state): State<AppState>,
    auth_user: AuthUser,
    Path(chat_id): Path<Uuid>,
) -> AppResult<Json<Value>> {
    let has_access = check_chat_access(&state, chat_id, auth_user.user_id).await?;
    if !has_access {
        return Err(AppError::Forbidden);
    }

    mark_messages_as_read(&state, chat_id, auth_user.user_id).await?;

    Ok(Json(json!({"success": true})))
}

async fn check_chat_access(state: &AppState, chat_id: Uuid, user_id: Uuid) -> AppResult<bool> {
    let chat = sqlx::query_as::<_, Chat>("SELECT * FROM chats WHERE id = $1")
        .bind(chat_id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Чат не найден".to_string()))?;

    match chat.chat_type {
        ChatType::Complex | ChatType::Building => {
            // Проверяем, что пользователь в этом ЖК
            if let Some(complex_id) = chat.complex_id {
                let in_complex: Option<(i32,)> = sqlx::query_as(
                    r#"
                    SELECT 1 FROM apartments
                    WHERE complex_id = $1 AND (owner_id = $2 OR resident_id = $2)
                    "#,
                )
                .bind(complex_id)
                .bind(user_id)
                .fetch_optional(&state.pool)
                .await?;
                Ok(in_complex.is_some())
            } else {
                Ok(false)
            }
        }
        ChatType::Private | ChatType::Support => {
            // Проверяем членство
            let is_member: Option<(i32,)> =
                sqlx::query_as("SELECT 1 FROM chat_members WHERE chat_id = $1 AND user_id = $2")
                    .bind(chat_id)
                    .bind(user_id)
                    .fetch_optional(&state.pool)
                    .await?;
            Ok(is_member.is_some())
        }
    }
}

async fn mark_messages_as_read(state: &AppState, chat_id: Uuid, user_id: Uuid) -> AppResult<()> {
    sqlx::query(
        r#"
        INSERT INTO message_reads (message_id, user_id)
        SELECT m.id, $2
        FROM chat_messages m
        LEFT JOIN message_reads r ON r.message_id = m.id AND r.user_id = $2
        WHERE m.chat_id = $1 AND r.id IS NULL AND m.sender_id != $2
        "#,
    )
    .bind(chat_id)
    .bind(user_id)
    .execute(&state.pool)
    .await?;

    // Обновляем last_read_at
    sqlx::query("UPDATE chat_members SET last_read_at = NOW() WHERE chat_id = $1 AND user_id = $2")
        .bind(chat_id)
        .bind(user_id)
        .execute(&state.pool)
        .await?;

    Ok(())
}
