use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MarketplaceCategory {
    pub id: Uuid,
    pub name: String,
    pub name_kz: Option<String>,
    pub slug: String,
    pub icon: Option<String>,
    pub parent_id: Option<Uuid>,
    pub sort_order: i32,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CategoryResponse {
    pub id: Uuid,
    pub name: String,
    pub name_kz: Option<String>,
    pub slug: String,
    pub icon: Option<String>,
    pub parent_id: Option<Uuid>,
}

impl From<MarketplaceCategory> for CategoryResponse {
    fn from(cat: MarketplaceCategory) -> Self {
        Self {
            id: cat.id,
            name: cat.name,
            name_kz: cat.name_kz,
            slug: cat.slug,
            icon: cat.icon,
            parent_id: cat.parent_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "listing_status", rename_all = "snake_case")]
pub enum ListingStatus {
    Draft,
    Active,
    Sold,
    Reserved,
    Archived,
}

impl Default for ListingStatus {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MarketplaceListing {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub seller_id: Uuid,
    pub category_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: Decimal,
    pub is_negotiable: bool,
    pub is_free: bool,
    pub condition: Option<String>,
    pub status: ListingStatus,
    pub views_count: i32,
    pub favorites_count: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ListingPhoto {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub url: String,
    pub is_main: bool,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ListingResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: Decimal,
    pub is_negotiable: bool,
    pub is_free: bool,
    pub condition: Option<String>,
    pub status: ListingStatus,
    pub category: CategoryResponse,
    pub seller: SellerInfo,
    pub photos: Vec<String>,
    pub views_count: i32,
    pub favorites_count: i32,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SellerInfo {
    pub id: Uuid,
    pub name: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateListingRequest {
    pub category_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub price: Decimal,
    pub is_negotiable: Option<bool>,
    pub is_free: Option<bool>,
    pub condition: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateListingRequest {
    pub category_id: Option<Uuid>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub price: Option<Decimal>,
    pub is_negotiable: Option<bool>,
    pub is_free: Option<bool>,
    pub condition: Option<String>,
    pub status: Option<ListingStatus>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ListingsQuery {
    pub category: Option<String>,
    pub query: Option<String>,
    pub min_price: Option<Decimal>,
    pub max_price: Option<Decimal>,
    pub condition: Option<String>,
    pub page: Option<i64>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, FromRow)]
pub struct ListingFavorite {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ListingMessage {
    pub id: Uuid,
    pub listing_id: Uuid,
    pub sender_id: Uuid,
    pub recipient_id: Uuid,
    pub message: String,
    pub is_read: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendMessageRequest {
    pub message: String,
}
