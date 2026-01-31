use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "complex_status", rename_all = "snake_case")]
pub enum ComplexStatus {
    Pending,
    Active,
    Inactive,
}

impl Default for ComplexStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Complex {
    pub id: Uuid,
    pub city_id: String,
    pub address_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub buildings_count: Option<i32>,
    pub floors_count: Option<i32>,
    pub apartments_count: Option<i32>,
    pub year_built: Option<i32>,
    pub has_parking: bool,
    pub has_underground_parking: bool,
    pub has_playground: bool,
    pub has_gym: bool,
    pub has_concierge: bool,
    pub has_security: bool,
    pub has_cctv: bool,
    pub status: ComplexStatus,
    pub verified_at: Option<DateTime<Utc>>,
    pub verified_by: Option<Uuid>,
    pub created_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow, ToSchema)]
pub struct ComplexPhoto {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub url: String,
    pub is_main: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ComplexResponse {
    pub id: Uuid,
    pub city_id: String,
    pub name: String,
    pub description: Option<String>,
    pub address: Option<String>,
    pub buildings_count: Option<i32>,
    pub floors_count: Option<i32>,
    pub apartments_count: Option<i32>,
    pub year_built: Option<i32>,
    pub amenities: ComplexAmenities,
    pub status: ComplexStatus,
    pub photos: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ComplexAmenities {
    pub has_parking: bool,
    pub has_underground_parking: bool,
    pub has_playground: bool,
    pub has_gym: bool,
    pub has_concierge: bool,
    pub has_security: bool,
    pub has_cctv: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateComplexRequest {
    pub city_id: String,
    pub address_id: Option<Uuid>,
    pub name: String,
    pub description: Option<String>,
    pub buildings_count: Option<i32>,
    pub floors_count: Option<i32>,
    pub apartments_count: Option<i32>,
    pub year_built: Option<i32>,
    pub has_parking: Option<bool>,
    pub has_underground_parking: Option<bool>,
    pub has_playground: Option<bool>,
    pub has_gym: Option<bool>,
    pub has_concierge: Option<bool>,
    pub has_security: Option<bool>,
    pub has_cctv: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchComplexQuery {
    pub city: Option<String>,
    pub query: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct JoinComplexRequest {
    pub apartment_number: String,
    pub building: Option<String>,
    pub is_owner: bool,
}
