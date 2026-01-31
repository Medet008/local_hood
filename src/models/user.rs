use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::Type, PartialEq, Eq, ToSchema)]
#[sqlx(type_name = "user_role", rename_all = "snake_case")]
pub enum UserRole {
    User,
    Resident,
    Owner,
    Council,
    Chairman,
    Moderator,
    Admin,
    SuperAdmin,
}

impl Default for UserRole {
    fn default() -> Self {
        Self::User
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow, ToSchema)]
pub struct User {
    pub id: Uuid,
    pub phone: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub email: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub is_verified: bool,
    pub is_blocked: bool,
    pub blocked_reason: Option<String>,
    pub blocked_at: Option<DateTime<Utc>>,
    pub last_login_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct UserPublic {
    pub id: Uuid,
    pub phone: String,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub avatar_url: Option<String>,
    pub role: UserRole,
    pub is_verified: bool,
}

impl From<User> for UserPublic {
    fn from(user: User) -> Self {
        Self {
            id: user.id,
            phone: mask_phone(&user.phone),
            first_name: user.first_name,
            last_name: user.last_name,
            avatar_url: user.avatar_url,
            role: user.role,
            is_verified: user.is_verified,
        }
    }
}

fn mask_phone(phone: &str) -> String {
    if phone.len() > 8 {
        format!("{}****{}", &phone[..4], &phone[phone.len() - 4..])
    } else {
        phone.to_string()
    }
}

#[derive(Debug, Clone, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub device_info: Option<String>,
    pub ip_address: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct SmsCode {
    pub id: Uuid,
    pub phone: String,
    pub code: String,
    pub attempts: i32,
    pub is_used: bool,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

// DTOs
#[derive(Debug, Deserialize, ToSchema)]
pub struct SendCodeRequest {
    pub phone: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyCodeRequest {
    pub phone: String,
    pub code: String,
    pub device_info: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub user: UserPublic,
    pub is_new_user: bool,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateUserRequest {
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub middle_name: Option<String>,
    pub email: Option<String>,
}
