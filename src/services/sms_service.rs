use crate::config::Config;
use crate::error::{AppError, AppResult};
use serde::{Deserialize, Serialize};

pub struct SmsService {
    config: Config,
    client: reqwest::Client,
}

#[derive(Debug, Serialize)]
struct MobizonRequest {
    recipient: String,
    text: String,
    from: String,
}

#[derive(Debug, Deserialize)]
struct MobizonResponse {
    code: i32,
    message: String,
}

impl SmsService {
    pub fn new(config: Config) -> Self {
        Self {
            config,
            client: reqwest::Client::new(),
        }
    }

    pub async fn send_code(&self, phone: &str, code: &str) -> AppResult<()> {
        if !self.config.sms_enabled {
            tracing::info!("SMS disabled. Code for {}: {}", phone, code);
            return Ok(());
        }

        let text = format!("Ваш код подтверждения LocalHood: {}. Никому не сообщайте этот код.", code);
        self.send_sms(phone, &text).await
    }

    pub async fn send_guest_entry_notification(
        &self,
        phone: &str,
        guest_name: &str,
        time: &str,
    ) -> AppResult<()> {
        if !self.config.sms_enabled {
            tracing::info!("SMS disabled. Guest entry notification for {}", phone);
            return Ok(());
        }

        let text = format!("LocalHood: Гость {} въехал в {}.", guest_name, time);
        self.send_sms(phone, &text).await
    }

    pub async fn send_overstay_notification(
        &self,
        phone: &str,
        guest_name: &str,
        minutes: i32,
    ) -> AppResult<()> {
        if !self.config.sms_enabled {
            tracing::info!("SMS disabled. Overstay notification for {}", phone);
            return Ok(());
        }

        let text = format!(
            "LocalHood: Гость {} не выехал. Прошло {} мин.",
            guest_name, minutes
        );
        self.send_sms(phone, &text).await
    }

    async fn send_sms(&self, phone: &str, text: &str) -> AppResult<()> {
        let url = format!(
            "https://api.mobizon.kz/service/message/sendsmsmessage?apiKey={}",
            self.config.sms_api_key
        );

        let params = [
            ("recipient", phone),
            ("text", text),
            ("from", &self.config.sms_sender),
        ];

        let response = self
            .client
            .post(&url)
            .form(&params)
            .send()
            .await
            .map_err(|e| AppError::Sms(e.to_string()))?;

        let status = response.status();
        let body = response
            .text()
            .await
            .map_err(|e| AppError::Sms(e.to_string()))?;

        if !status.is_success() {
            tracing::error!("SMS API error: {} - {}", status, body);
            return Err(AppError::Sms(format!("SMS API error: {}", status)));
        }

        let result: MobizonResponse =
            serde_json::from_str(&body).map_err(|e| AppError::Sms(e.to_string()))?;

        if result.code != 0 {
            tracing::error!("SMS send failed: {}", result.message);
            return Err(AppError::Sms(result.message));
        }

        tracing::info!("SMS sent to {}", phone);
        Ok(())
    }
}
