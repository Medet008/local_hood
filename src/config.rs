use std::env;

#[derive(Clone, Debug)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub jwt_secret: String,
    pub jwt_access_expiry: i64,
    pub jwt_refresh_expiry: i64,
    pub sms_api_key: String,
    pub sms_sender: String,
    pub sms_enabled: bool,
    pub minio_endpoint: String,
    pub minio_access_key: String,
    pub minio_secret_key: String,
    pub minio_bucket: String,
    pub minio_public_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Result<Self, env::VarError> {
        Ok(Self {
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            database_url: env::var("DATABASE_URL")?,
            jwt_secret: env::var("JWT_SECRET")?,
            jwt_access_expiry: env::var("JWT_ACCESS_EXPIRY")
                .unwrap_or_else(|_| "900".to_string())
                .parse()
                .unwrap_or(900),
            jwt_refresh_expiry: env::var("JWT_REFRESH_EXPIRY")
                .unwrap_or_else(|_| "2592000".to_string())
                .parse()
                .unwrap_or(2592000),
            sms_api_key: env::var("SMS_API_KEY").unwrap_or_default(),
            sms_sender: env::var("SMS_SENDER").unwrap_or_else(|_| "LocalHood".to_string()),
            sms_enabled: env::var("SMS_ENABLED")
                .unwrap_or_else(|_| "false".to_string())
                .parse()
                .unwrap_or(false),
            minio_endpoint: env::var("MINIO_ENDPOINT")
                .unwrap_or_else(|_| "http://localhost:9000".to_string()),
            minio_access_key: env::var("MINIO_ACCESS_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            minio_secret_key: env::var("MINIO_SECRET_KEY")
                .unwrap_or_else(|_| "minioadmin".to_string()),
            minio_bucket: env::var("MINIO_BUCKET")
                .unwrap_or_else(|_| "localhood".to_string()),
            minio_public_url: env::var("MINIO_PUBLIC_URL").ok(),
        })
    }
}
