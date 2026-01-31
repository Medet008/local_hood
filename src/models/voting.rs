use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "voting_type", rename_all = "snake_case")]
pub enum VotingType {
    SingleChoice,
    MultipleChoice,
    YesNo,
}

impl Default for VotingType {
    fn default() -> Self {
        Self::SingleChoice
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "voting_status", rename_all = "snake_case")]
pub enum VotingStatus {
    Draft,
    Active,
    Closed,
    Cancelled,
}

impl Default for VotingStatus {
    fn default() -> Self {
        Self::Draft
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Voting {
    pub id: Uuid,
    pub complex_id: Uuid,
    pub osi_id: Option<Uuid>,
    pub title: String,
    pub description: Option<String>,
    pub voting_type: VotingType,
    pub status: VotingStatus,
    pub requires_owner: bool,
    pub quorum_percent: i32,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub created_by: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct VotingOption {
    pub id: Uuid,
    pub voting_id: Uuid,
    pub text: String,
    pub sort_order: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct Vote {
    pub id: Uuid,
    pub voting_id: Uuid,
    pub option_id: Uuid,
    pub user_id: Uuid,
    pub apartment_id: Option<Uuid>,
    pub vote_weight: Decimal,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VotingResponse {
    pub id: Uuid,
    pub title: String,
    pub description: Option<String>,
    pub voting_type: VotingType,
    pub status: VotingStatus,
    pub requires_owner: bool,
    pub quorum_percent: i32,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub options: Vec<VotingOptionResponse>,
    pub total_votes: i32,
    pub total_weight: Decimal,
    pub user_voted: bool,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VotingOptionResponse {
    pub id: Uuid,
    pub text: String,
    pub votes_count: i32,
    pub votes_weight: Decimal,
    pub percentage: f64,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateVotingRequest {
    pub title: String,
    pub description: Option<String>,
    pub voting_type: Option<VotingType>,
    pub requires_owner: Option<bool>,
    pub quorum_percent: Option<i32>,
    pub starts_at: DateTime<Utc>,
    pub ends_at: DateTime<Utc>,
    pub options: Vec<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CastVoteRequest {
    pub option_id: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct VotingDocument {
    pub id: Uuid,
    pub voting_id: Uuid,
    pub title: String,
    pub file_url: String,
    pub created_at: DateTime<Utc>,
}
