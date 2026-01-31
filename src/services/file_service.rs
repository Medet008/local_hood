use crate::config::Config;
use crate::error::{AppError, AppResult};
use aws_config::BehaviorVersion;
use aws_sdk_s3::config::{Credentials, Region};
use aws_sdk_s3::primitives::ByteStream;
use aws_sdk_s3::Client;
use uuid::Uuid;

pub struct FileService {
    client: Client,
    bucket: String,
    public_url: Option<String>,
}

impl FileService {
    pub async fn new(config: &Config) -> AppResult<Self> {
        let credentials = Credentials::new(
            &config.minio_access_key,
            &config.minio_secret_key,
            None,
            None,
            "localhood",
        );

        let s3_config = aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::latest())
            .credentials_provider(credentials)
            .region(Region::new("us-east-1"))
            .endpoint_url(&config.minio_endpoint)
            .force_path_style(true)
            .build();

        let client = Client::from_conf(s3_config);

        Ok(Self {
            client,
            bucket: config.minio_bucket.clone(),
            public_url: config.minio_public_url.clone(),
        })
    }

    pub async fn upload_file(
        &self,
        folder: &str,
        file_name: &str,
        content_type: &str,
        data: Vec<u8>,
    ) -> AppResult<String> {
        let extension = file_name
            .rsplit('.')
            .next()
            .unwrap_or("bin");

        let key = format!("{}/{}.{}", folder, Uuid::new_v4(), extension);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(ByteStream::from(data))
            .content_type(content_type)
            .send()
            .await
            .map_err(|e| AppError::File(e.to_string()))?;

        let url = match &self.public_url {
            Some(base_url) => format!("{}/{}/{}", base_url, self.bucket, key),
            None => format!("/{}/{}", self.bucket, key),
        };

        Ok(url)
    }

    pub async fn delete_file(&self, key: &str) -> AppResult<()> {
        self.client
            .delete_object()
            .bucket(&self.bucket)
            .key(key)
            .send()
            .await
            .map_err(|e| AppError::File(e.to_string()))?;

        Ok(())
    }

    pub fn get_key_from_url(&self, url: &str) -> Option<String> {
        let prefix = format!("/{}/", self.bucket);
        if let Some(pos) = url.find(&prefix) {
            Some(url[pos + prefix.len()..].to_string())
        } else {
            None
        }
    }
}

pub fn validate_image_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "image/jpeg" | "image/png" | "image/gif" | "image/webp"
    )
}

pub fn validate_document_content_type(content_type: &str) -> bool {
    matches!(
        content_type,
        "application/pdf"
            | "application/msword"
            | "application/vnd.openxmlformats-officedocument.wordprocessingml.document"
            | "image/jpeg"
            | "image/png"
    )
}

pub const MAX_IMAGE_SIZE: usize = 10 * 1024 * 1024; // 10MB
pub const MAX_DOCUMENT_SIZE: usize = 50 * 1024 * 1024; // 50MB
