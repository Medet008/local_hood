use chrono::{DateTime, NaiveDate, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Osi {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub name: String,
    pub bin: Option<String>,
    pub chairman_id: Option<Uuid>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub bank_name: Option<String>,
    pub bank_bik: Option<String>,
    pub bank_account: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OsiResponse {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub name: String,
    pub bin: Option<String>,
    pub chairman: Option<ChairmanInfo>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ChairmanInfo {
    pub id: Uuid,
    pub name: String,
    pub phone: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateOsiRequest {
    pub name: Option<String>,
    pub bin: Option<String>,
    pub phone: Option<String>,
    pub email: Option<String>,
    pub address: Option<String>,
    pub bank_name: Option<String>,
    pub bank_bik: Option<String>,
    pub bank_account: Option<String>,
}

// Совет дома
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "council_position", rename_all = "snake_case")]
pub enum CouncilPosition {
    Chairman,
    DeputyChairman,
    Secretary,
    Treasurer,
    Member,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct CouncilMember {
    pub id: Uuid,
    pub osi_id: Uuid,
    pub user_id: Uuid,
    pub position: CouncilPosition,
    pub responsibilities: Option<String>,
    pub appointed_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CouncilMemberResponse {
    pub id: Uuid,
    pub user_id: Uuid,
    pub user_name: String,
    pub user_phone: String,
    pub position: CouncilPosition,
    pub responsibilities: Option<String>,
    pub appointed_at: DateTime<Utc>,
    pub is_active: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct AddCouncilMemberRequest {
    pub user_id: Uuid,
    pub position: CouncilPosition,
    pub responsibilities: Option<String>,
}

// Работники
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "worker_role", rename_all = "snake_case")]
pub enum WorkerRole {
    Accountant,
    Manager,
    Guard,
    Cleaner,
    Plumber,
    Electrician,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct OsiWorker {
    pub id: Uuid,
    pub osi_id: Uuid,
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub phone: Option<String>,
    pub role: WorkerRole,
    pub position_title: Option<String>,
    pub salary: Option<Decimal>,
    pub hired_at: Option<NaiveDate>,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateWorkerRequest {
    pub first_name: String,
    pub last_name: String,
    pub middle_name: Option<String>,
    pub phone: Option<String>,
    pub role: WorkerRole,
    pub position_title: Option<String>,
    pub salary: Option<Decimal>,
    pub hired_at: Option<NaiveDate>,
}

// Документы
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "document_type", rename_all = "snake_case")]
pub enum DocumentType {
    Charter,
    Protocol,
    Contract,
    Report,
    Act,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct OsiDocument {
    pub id: Uuid,
    pub osi_id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub document_type: DocumentType,
    pub file_url: String,
    pub file_size: Option<i32>,
    pub uploaded_by: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct OsiDocumentResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub document_type: DocumentType,
    pub file_url: String,
    pub file_size: Option<i32>,
    pub uploaded_by_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

// Заявки на председателя
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "chairman_application_status", rename_all = "snake_case")]
pub enum ChairmanApplicationStatus {
    Pending,
    Approved,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct ChairmanApplication {
    pub id: Uuid,
    pub user_id: Uuid,
    pub complex_id: Uuid,
    pub document_url: Option<String>,
    pub motivation: Option<String>,
    pub status: ChairmanApplicationStatus,
    pub reviewed_by: Option<Uuid>,
    pub reviewed_at: Option<DateTime<Utc>>,
    pub rejection_reason: Option<String>,
    pub created_at: DateTime<Utc>,
}
