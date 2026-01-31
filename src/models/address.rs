use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Address {
    pub id: Uuid,
    pub city_id: String,
    pub district: Option<String>,
    pub street: String,
    pub building: String,
    pub postal_code: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AddressResponse {
    pub id: Uuid,
    pub city_id: String,
    pub city_name: Option<String>,
    pub district: Option<String>,
    pub street: String,
    pub building: String,
    pub full_address: String,
}

impl Address {
    pub fn full_address(&self, city_name: &str) -> String {
        format!("г. {}, {}, {}", city_name, self.street, self.building)
    }
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAddressRequest {
    pub city_id: String,
    pub district: Option<String>,
    pub street: String,
    pub building: String,
    pub postal_code: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SearchAddressQuery {
    pub city: String,
    pub query: String,
}
