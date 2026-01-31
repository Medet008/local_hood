use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Apartment {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub building: Option<String>,
    pub entrance: Option<String>,
    pub number: String,
    pub floor: Option<i32>,
    pub area: Option<Decimal>,
    pub rooms_count: Option<i32>,
    pub owner_id: Option<Uuid>,
    pub resident_id: Option<Uuid>,
    pub is_ownership_verified: bool,
    pub ownership_document_url: Option<String>,
    pub verified_at: Option<DateTime<Utc>>,
    pub verified_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ApartmentResponse {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub complex_name: Option<String>,
    pub building: Option<String>,
    pub entrance: Option<String>,
    pub number: String,
    pub floor: Option<i32>,
    pub area: Option<Decimal>,
    pub rooms_count: Option<i32>,
    pub is_owner: bool,
    pub is_resident: bool,
    pub is_ownership_verified: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "join_request_status", rename_all = "snake_case")]
pub enum JoinRequestStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct JoinRequest {
    pub id: Uuid,
    pub user_id: Uuid,
    pub complex_id: Uuid,
    pub apartment_id: Option<Uuid>,
    pub apartment_number: String,
    pub building: Option<String>,
    pub is_owner: bool,
    pub document_url: Option<String>,
    pub status: JoinRequestStatus,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct JoinRequestResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: Option<String>,
    pub user_phone: Option<String>,
    pub complex_id: Uuid,
    pub apartment_number: String,
    pub building: Option<String>,
    pub is_owner: bool,
    pub document_url: Option<String>,
    pub status: JoinRequestStatus,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ReviewJoinRequestRequest {
    pub approved: bool,
    pub rejection_reason: Option<String>,
}
