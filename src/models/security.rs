use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

// Статус гостевого доступа
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "guest_access_status", rename_all = "snake_case")]
pub enum GuestAccessStatus {
    Pending,
    Active,
    Expired,
    Completed,
    Cancelled,
}

impl Default for GuestAccessStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct GuestAccess {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub created_by: Uuid,
    pub guest_name: Option<String>,
    pub guest_phone: Option<String>,
    pub vehicle_number: Option<String>,
    pub access_code: String,
    pub qr_code_url: Option<String>,
    pub duration_minutes: i32,
    pub expires_at: DateTime<Utc>,
    pub entered_at: Option<DateTime<Utc>>,
    pub exited_at: Option<DateTime<Utc>>,
    pub status: GuestAccessStatus,
    pub owner_notified: bool,
    pub chairman_notified: bool,
    pub overstay_notified: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct GuestAccessResponse {
    pub id: Uuid,
    pub guest_name: Option<String>,
    pub guest_phone: Option<String>,
    pub vehicle_number: Option<String>,
    pub access_code: String,
    pub qr_code_url: Option<String>,
    pub duration_minutes: i32,
    pub expires_at: DateTime<Utc>,
    pub entered_at: Option<DateTime<Utc>>,
    pub exited_at: Option<DateTime<Utc>>,
    pub status: GuestAccessStatus,
    pub created_at: DateTime<Utc>,
}

impl From<GuestAccess> for GuestAccessResponse {
    fn from(ga: GuestAccess) -> Self {
        Self {
            id: ga.id,
            guest_name: ga.guest_name,
            guest_phone: ga.guest_phone,
            vehicle_number: ga.vehicle_number,
            access_code: ga.access_code,
            qr_code_url: ga.qr_code_url,
            duration_minutes: ga.duration_minutes,
            expires_at: ga.expires_at,
            entered_at: ga.entered_at,
            exited_at: ga.exited_at,
            status: ga.status,
            created_at: ga.created_at,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateGuestAccessRequest {
    pub guest_name: Option<String>,
    pub guest_phone: Option<String>,
    pub vehicle_number: Option<String>,
    pub duration_minutes: Option<i32>,
}

// Шлагбаумы
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Barrier {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub name: String,
    pub location: Option<String>,
    pub device_type: Option<String>,
    pub device_ip: Option<String>,
    pub device_port: Option<i32>,
    pub api_key: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

// Действие шлагбаума
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "barrier_action", rename_all = "snake_case")]
pub enum BarrierAction {
    Entry,
    Exit,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BarrierAccessLog {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub barrier_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub guest_access_id: Option<Uuid>,
    pub action: BarrierAction,
    pub vehicle_number: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BarrierAccessLogResponse {
    pub id: Uuid,
    pub action: BarrierAction,
    pub vehicle_number: Option<String>,
    pub user_name: Option<String>,
    pub guest_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BarrierEntryRequest {
    pub access_code: Option<String>,
    pub vehicle_number: Option<String>,
    pub barrier_id: Option<Uuid>,
}

// Камеры
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Camera {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub name: String,
    pub location: Option<String>,
    pub stream_url: Option<String>,
    pub cloud_provider: Option<String>,
    pub cloud_camera_id: Option<String>,
    pub is_public: bool,
    pub requires_owner: bool,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CameraResponse {
    pub id: Uuid,
    pub name: String,
    pub location: Option<String>,
    pub is_active: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CameraStreamResponse {
    pub id: Uuid,
    pub name: String,
    pub stream_url: String,
}

// Домофоны
#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Intercom {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub name: String,
    pub location: Option<String>,
    pub device_type: Option<String>,
    pub device_id: Option<String>,
    pub sip_address: Option<String>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "intercom_call_status", rename_all = "snake_case")]
pub enum IntercomCallStatus {
    Missed,
    Answered,
    Opened,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct IntercomCall {
    pub id: Uuid,
    pub intercom_id: Uuid,
    pub apartment_id: Option<Uuid>,
    pub user_id: Option<Uuid>,
    pub status: IntercomCallStatus,
    pub duration_seconds: Option<i32>,
    pub snapshot_url: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct IntercomCallResponse {
    pub id: Uuid,
    pub intercom_name: String,
    pub status: IntercomCallStatus,
    pub duration_seconds: Option<i32>,
    pub snapshot_url: Option<String>,
    pub created_at: DateTime<Utc>,
}
