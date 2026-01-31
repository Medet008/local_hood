use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "notification_type", rename_all = "snake_case")]
pub enum NotificationType {
    Announcement,
    Voting,
    Bill,
    Payment,
    GuestAccess,
    Maintenance,
    Security,
    Chat,
    Marketplace,
    System,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Notification {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: Option<String>,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,
    pub read_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct NotificationResponse {
    pub id: Uuid,
    pub notification_type: NotificationType,
    pub title: String,
    pub body: Option<String>,
    pub data: Option<serde_json::Value>,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

impl From<Notification> for NotificationResponse {
    fn from(n: Notification) -> Self {
        Self {
            id: n.id,
            notification_type: n.notification_type,
            title: n.title,
            body: n.body,
            data: n.data,
            is_read: n.is_read,
            created_at: n.created_at,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct PushToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token: String,
    pub platform: String,
    pub device_id: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterPushTokenRequest {
    pub token: String,
    pub platform: String,
    pub device_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct NotificationsQuery {
    pub unread_only: Option<bool>,
    pub notification_type: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}
