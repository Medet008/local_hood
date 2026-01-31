use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "utility_type", rename_all = "snake_case")]
pub enum UtilityType {
    Electricity,
    ColdWater,
    HotWater,
    Heating,
    Gas,
    Maintenance,
    Garbage,
    Elevator,
    Intercom,
    Parking,
    Security,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Meter {
    pub id: Uuid,
    pub apartment_id: Uuid,
    pub utility_type: UtilityType,
    pub serial_number: Option<String>,
    pub installation_date: Option<NaiveDate>,
    pub verification_date: Option<NaiveDate>,
    pub next_verification_date: Option<NaiveDate>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MeterResponse {
    pub id: Uuid,
    pub utility_type: UtilityType,
    pub serial_number: Option<String>,
    pub last_reading: Option<Decimal>,
    pub last_reading_date: Option<NaiveDate>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct MeterReading {
    pub id: Uuid,
    pub meter_id: Uuid,
    pub apartment_id: Uuid,
    pub value: Decimal,
    pub previous_value: Option<Decimal>,
    pub consumption: Option<Decimal>,
    pub reading_date: NaiveDate,
    pub submitted_by: Option<Uuid>,
    pub photo_url: Option<String>,
    pub is_verified: bool,
    pub verified_by: Option<Uuid>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SubmitReadingRequest {
    pub meter_id: Uuid,
    pub value: Decimal,
    pub photo_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "bill_status", rename_all = "snake_case")]
pub enum BillStatus {
    Pending,
    Paid,
    Overdue,
    Cancelled,
}

impl Default for BillStatus {
    fn default() -> Self {
        Self::Pending
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Bill {
    pub id: Uuid,
    pub apartment_id: Uuid,
    pub complex_id: Uuid,
    pub period_start: NaiveDate,
    pub period_end: NaiveDate,
    pub amount: Decimal,
    pub debt: Decimal,
    pub penalty: Decimal,
    pub total_amount: Decimal,
    pub status: BillStatus,
    pub due_date: NaiveDate,
    pub paid_at: Option<DateTime<Utc>>,
    pub paid_amount: Option<Decimal>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct BillItem {
    pub id: Uuid,
    pub bill_id: Uuid,
    pub utility_type: UtilityType,
    pub description: Option<String>,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub rate: Option<Decimal>,
    pub amount: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BillResponse {
    pub id: Uuid,
    pub period: String,
    pub amount: Decimal,
    pub debt: Decimal,
    pub penalty: Decimal,
    pub total_amount: Decimal,
    pub status: BillStatus,
    pub due_date: NaiveDate,
    pub items: Vec<BillItemResponse>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct BillItemResponse {
    pub utility_type: UtilityType,
    pub description: Option<String>,
    pub quantity: Option<Decimal>,
    pub unit: Option<String>,
    pub rate: Option<Decimal>,
    pub amount: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "payment_status", rename_all = "snake_case")]
pub enum PaymentStatus {
    Pending,
    Processing,
    Completed,
    Failed,
    Refunded,
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "payment_method", rename_all = "snake_case")]
pub enum PaymentMethod {
    Card,
    Kaspi,
    Halyk,
    BankTransfer,
    Cash,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Payment {
    pub id: Uuid,
    pub bill_id: Option<Uuid>,
    pub apartment_id: Uuid,
    pub user_id: Uuid,
    pub amount: Decimal,
    pub method: PaymentMethod,
    pub status: PaymentStatus,
    pub external_id: Option<String>,
    pub payment_url: Option<String>,
    pub completed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreatePaymentRequest {
    pub bill_id: Uuid,
    pub method: PaymentMethod,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PaymentResponse {
    pub id: Uuid,
    pub amount: Decimal,
    pub method: PaymentMethod,
    pub status: PaymentStatus,
    pub payment_url: Option<String>,
    pub created_at: DateTime<Utc>,
}
