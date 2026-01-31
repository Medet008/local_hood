use crate::config::Config;
use crate::error::{AppError, AppResult};
use crate::models::{User, UserRole};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,  // user_id
    pub role: String,
    pub exp: i64,
    pub iat: i64,
    pub token_type: String,
}

pub struct AuthService {
    config: Config,
}

impl AuthService {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn generate_access_token(&self, user: &User) -> AppResult<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.jwt_access_expiry);

        let claims = Claims {
            sub: user.id.to_string(),
            role: format!("{:?}", user.role).to_lowercase(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type: "access".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(AppError::from)
    }

    pub fn generate_refresh_token(&self, user: &User) -> AppResult<String> {
        let now = Utc::now();
        let exp = now + Duration::seconds(self.config.jwt_refresh_expiry);

        let claims = Claims {
            sub: user.id.to_string(),
            role: format!("{:?}", user.role).to_lowercase(),
            exp: exp.timestamp(),
            iat: now.timestamp(),
            token_type: "refresh".to_string(),
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.config.jwt_secret.as_bytes()),
        )
        .map_err(AppError::from)
    }

    pub fn verify_token(&self, token: &str) -> AppResult<Claims> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.config.jwt_secret.as_bytes()),
            &Validation::default(),
        )?;

        Ok(token_data.claims)
    }

    pub fn generate_sms_code() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(100000..999999))
    }

    pub fn generate_access_code() -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        format!("{:06}", rng.gen_range(100000..999999))
    }

    pub async fn get_user_by_id(pool: &PgPool, user_id: Uuid) -> AppResult<User> {
        sqlx::query_as::<_, User>("SELECT * FROM users WHERE id = $1")
            .bind(user_id)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Пользователь не найден".to_string()))
    }

    pub async fn get_user_by_phone(pool: &PgPool, phone: &str) -> AppResult<Option<User>> {
        let user = sqlx::query_as::<_, User>("SELECT * FROM users WHERE phone = $1")
            .bind(phone)
            .fetch_optional(pool)
            .await?;
        Ok(user)
    }

    pub async fn create_user(pool: &PgPool, phone: &str) -> AppResult<User> {
        let user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (phone, role, is_verified)
            VALUES ($1, $2, false)
            RETURNING *
            "#,
        )
        .bind(phone)
        .bind(UserRole::User)
        .fetch_one(pool)
        .await?;

        Ok(user)
    }

    pub async fn save_sms_code(pool: &PgPool, phone: &str, code: &str) -> AppResult<()> {
        let expires_at = Utc::now() + Duration::minutes(5);

        sqlx::query(
            r#"
            INSERT INTO sms_codes (phone, code, expires_at)
            VALUES ($1, $2, $3)
            "#,
        )
        .bind(phone)
        .bind(code)
        .bind(expires_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn verify_sms_code(pool: &PgPool, phone: &str, code: &str) -> AppResult<bool> {
        let result = sqlx::query_as::<_, (i32,)>(
            r#"
            UPDATE sms_codes
            SET is_used = true, attempts = attempts + 1
            WHERE phone = $1
              AND code = $2
              AND is_used = false
              AND expires_at > NOW()
              AND attempts < 3
            RETURNING 1
            "#,
        )
        .bind(phone)
        .bind(code)
        .fetch_optional(pool)
        .await?;

        Ok(result.is_some())
    }

    pub async fn save_refresh_token(
        pool: &PgPool,
        user_id: Uuid,
        token_hash: &str,
        device_info: Option<&str>,
        ip_address: Option<&str>,
        expires_at: chrono::DateTime<Utc>,
    ) -> AppResult<()> {
        sqlx::query(
            r#"
            INSERT INTO refresh_tokens (user_id, token_hash, device_info, ip_address, expires_at)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(user_id)
        .bind(token_hash)
        .bind(device_info)
        .bind(ip_address)
        .bind(expires_at)
        .execute(pool)
        .await?;

        Ok(())
    }

    pub async fn delete_refresh_token(pool: &PgPool, token_hash: &str) -> AppResult<()> {
        sqlx::query("DELETE FROM refresh_tokens WHERE token_hash = $1")
            .bind(token_hash)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub async fn update_last_login(pool: &PgPool, user_id: Uuid) -> AppResult<()> {
        sqlx::query("UPDATE users SET last_login_at = NOW() WHERE id = $1")
            .bind(user_id)
            .execute(pool)
            .await?;

        Ok(())
    }

    pub fn hash_token(token: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        token.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}

pub fn normalize_phone(phone: &str) -> String {
    let digits: String = phone.chars().filter(|c| c.is_ascii_digit()).collect();

    if digits.starts_with("8") && digits.len() == 11 {
        format!("+7{}", &digits[1..])
    } else if digits.starts_with("7") && digits.len() == 11 {
        format!("+{}", digits)
    } else if digits.len() == 10 {
        format!("+7{}", digits)
    } else {
        format!("+{}", digits)
    }
}

pub fn validate_kz_phone(phone: &str) -> bool {
    let normalized = normalize_phone(phone);
    normalized.starts_with("+7") && normalized.len() == 12
}
