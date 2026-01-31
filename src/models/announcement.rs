use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "announcement_category", rename_all = "snake_case")]
pub enum AnnouncementCategory {
    General,
    Maintenance,
    Emergency,
    Event,
    Financial,
    Voting,
}

impl Default for AnnouncementCategory {
    fn default() -> Self {
        Self::General
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "announcement_priority", rename_all = "snake_case")]
pub enum AnnouncementPriority {
    Low,
    Normal,
    High,
    Urgent,
}

impl Default for AnnouncementPriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Announcement {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub title: String,
    pub content: String,
    pub category: AnnouncementCategory,
    pub priority: AnnouncementPriority,
    pub image_url: Option<String>,
    pub is_published: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub expires_at: Option<DateTime<Utc>>,
    pub author_id: Uuid,
    pub views_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AnnouncementResponse {
    pub id: Uuid,
    pub title: String,
    pub content: String,
    pub category: AnnouncementCategory,
    pub priority: AnnouncementPriority,
    pub image_url: Option<String>,
    pub author_name: Option<String>,
    pub views_count: i32,
    pub is_read: bool,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAnnouncementRequest {
    pub title: String,
    pub content: String,
    pub category: Option<AnnouncementCategory>,
    pub priority: Option<AnnouncementPriority>,
    pub image_url: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateAnnouncementRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub category: Option<AnnouncementCategory>,
    pub priority: Option<AnnouncementPriority>,
    pub image_url: Option<String>,
    pub is_published: Option<bool>,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct AnnouncementRead {
    pub id: Uuid,
    pub announcement_id: Uuid,
    pub user_id: Uuid,
    pub read_at: DateTime<Utc>,
}
