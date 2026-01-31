use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "chat_type", rename_all = "snake_case")]
pub enum ChatType {
    Complex,
    Building,
    Private,
    Support,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Chat {
    pub id: Uuid,
    pub complex_id: Option<Uuid>,
    pub chat_type: ChatType,
    pub name: Option<String>,
    pub is_private: bool,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ChatMember {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub user_id: Uuid,
    pub is_admin: bool,
    pub is_muted: bool,
    pub joined_at: DateTime<Utc>,
    pub last_read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ChatMessage {
    pub id: Uuid,
    pub chat_id: Uuid,
    pub sender_id: Uuid,
    pub content: String,
    pub attachment_url: Option<String>,
    pub attachment_type: Option<String>,
    pub reply_to_id: Option<Uuid>,
    pub is_edited: bool,
    pub edited_at: Option<DateTime<Utc>>,
    pub is_deleted: bool,
    pub deleted_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatResponse {
    pub id: Uuid,
    pub chat_type: ChatType,
    pub name: Option<String>,
    pub last_message: Option<MessagePreview>,
    pub unread_count: i32,
    pub members_count: i32,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MessagePreview {
    pub content: String,
    pub sender_name: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChatMessageResponse {
    pub id: Uuid,
    pub sender: SenderInfo,
    pub content: String,
    pub attachment_url: Option<String>,
    pub attachment_type: Option<String>,
    pub reply_to: Option<Box<ChatMessageResponse>>,
    pub is_edited: bool,
    pub is_deleted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SenderInfo {
    pub id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendChatMessageRequest {
    pub content: String,
    pub attachment_url: Option<String>,
    pub attachment_type: Option<String>,
    pub reply_to_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePrivateChatRequest {
    pub user_id: Uuid,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MessagesQuery {
    pub before: Option<Uuid>,
    pub limit: Option<i64>,
}
