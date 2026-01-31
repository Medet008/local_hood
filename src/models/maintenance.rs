use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "maintenance_category", rename_all = "snake_case")]
pub enum MaintenanceCategory {
    Plumbing,
    Electrical,
    Heating,
    Elevator,
    CommonArea,
    Facade,
    Roof,
    Parking,
    Landscaping,
    Security,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "maintenance_status", rename_all = "snake_case")]
pub enum MaintenanceStatus {
    New,
    InProgress,
    WaitingParts,
    Completed,
    Rejected,
    Cancelled,
}

impl Default for MaintenanceStatus {
    fn default() -> Self {
        Self::New
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "maintenance_priority", rename_all = "snake_case")]
pub enum MaintenancePriority {
    Low,
    Normal,
    High,
    Emergency,
}

impl Default for MaintenancePriority {
    fn default() -> Self {
        Self::Normal
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MaintenanceRequest {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub apartment_id: Option<Uuid>,
    pub requester_id: Uuid,
    pub category: MaintenanceCategory,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub priority: MaintenancePriority,
    pub status: MaintenanceStatus,
    pub assigned_to: Option<Uuid>,
    pub assigned_at: Option<DateTime<Utc>>,
    pub completed_at: Option<DateTime<Utc>>,
    pub completion_notes: Option<String>,
    pub rating: Option<i32>,
    pub rating_comment: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MaintenancePhoto {
    pub id: Uuid,
    pub request_id: Uuid,
    pub url: String,
    pub is_before: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MaintenanceComment {
    pub id: Uuid,
    pub request_id: Uuid,
    pub user_id: Uuid,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MaintenanceRequestResponse {
    pub id: Uuid,
    pub category: MaintenanceCategory,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub priority: MaintenancePriority,
    pub status: MaintenanceStatus,
    pub assigned_to_name: Option<String>,
    pub photos: Vec<MaintenancePhotoResponse>,
    pub comments_count: i32,
    pub rating: Option<i32>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MaintenancePhotoResponse {
    pub url: String,
    pub is_before: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMaintenanceRequest {
    pub category: MaintenanceCategory,
    pub title: String,
    pub description: Option<String>,
    pub location: Option<String>,
    pub priority: Option<MaintenancePriority>,
    pub apartment_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMaintenanceStatusRequest {
    pub status: MaintenanceStatus,
    pub completion_notes: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RateMaintenanceRequest {
    pub rating: i32,
    pub comment: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddMaintenanceCommentRequest {
    pub content: String,
}
