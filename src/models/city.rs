use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct City {
    pub id: String,
    pub name: String,
    pub name_kz: Option<String>,
    pub region: Option<String>,
    pub latitude: Option<Decimal>,
    pub longitude: Option<Decimal>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CityResponse {
    pub id: String,
    pub name: String,
    pub name_kz: Option<String>,
    pub region: Option<String>,
}

impl From<City> for CityResponse {
    fn from(city: City) -> Self {
        Self {
            id: city.id,
            name: city.name,
            name_kz: city.name_kz,
            region: city.region,
        }
    }
}
