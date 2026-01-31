use crate::error::{AppError, AppResult};
use crate::models::{BarrierAction, GuestAccess, GuestAccessStatus};
use crate::services::{AuthService, SmsService};
use chrono::{Duration, Utc};
use sqlx::PgPool;
use uuid::Uuid;

pub struct BarrierService {
    sms_service: SmsService,
}

impl BarrierService {
    pub fn new(sms_service: SmsService) -> Self {
        Self { sms_service }
    }

    pub async fn create_guest_access(
        &self,
        pool: &PgPool,
        complex_id: Uuid,
        user_id: Uuid,
        guest_name: Option<String>,
        guest_phone: Option<String>,
        vehicle_number: Option<String>,
        duration_minutes: i32,
    ) -> AppResult<GuestAccess> {
        let access_code = AuthService::generate_access_code();
        let expires_at = Utc::now() + Duration::minutes(duration_minutes as i64);

        let guest_access = sqlx::query_as::<_, GuestAccess>(
            r#"
            INSERT INTO guest_access
                (complex_id, created_by, guest_name, guest_phone, vehicle_number,
                 access_code, duration_minutes, expires_at, status)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
            RETURNING *
            "#,
        )
        .bind(complex_id)
        .bind(user_id)
        .bind(&guest_name)
        .bind(&guest_phone)
        .bind(&vehicle_number)
        .bind(&access_code)
        .bind(duration_minutes)
        .bind(expires_at)
        .bind(GuestAccessStatus::Pending)
        .fetch_one(pool)
        .await?;

        Ok(guest_access)
    }

    pub async fn process_entry(
        &self,
        pool: &PgPool,
        access_code: &str,
        vehicle_number: Option<&str>,
        barrier_id: Option<Uuid>,
    ) -> AppResult<GuestAccess> {
        // Найти активный гостевой доступ
        let guest_access = sqlx::query_as::<_, GuestAccess>(
            r#"
            SELECT * FROM guest_access
            WHERE access_code = $1
              AND status = 'pending'
              AND expires_at > NOW()
            "#,
        )
        .bind(access_code)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Код доступа не найден или истёк".to_string()))?;

        // Обновить статус
        let updated = sqlx::query_as::<_, GuestAccess>(
            r#"
            UPDATE guest_access
            SET status = 'active', entered_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(guest_access.id)
        .fetch_one(pool)
        .await?;

        // Записать лог
        sqlx::query(
            r#"
            INSERT INTO barrier_access_logs
                (complex_id, barrier_id, guest_access_id, action, vehicle_number)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(guest_access.complex_id)
        .bind(barrier_id)
        .bind(guest_access.id)
        .bind(BarrierAction::Entry)
        .bind(vehicle_number.or(guest_access.vehicle_number.as_deref()))
        .execute(pool)
        .await?;

        // Уведомить владельца
        if let Some(owner_phone) = self.get_owner_phone(pool, guest_access.created_by).await? {
            let guest_name = updated.guest_name.clone().unwrap_or_else(|| "Гость".to_string());
            let time = Utc::now().format("%H:%M").to_string();

            if let Err(e) = self.sms_service.send_guest_entry_notification(&owner_phone, &guest_name, &time).await {
                tracing::error!("Failed to send entry notification: {}", e);
            }

            // Отметить, что владелец уведомлён
            sqlx::query("UPDATE guest_access SET owner_notified = true WHERE id = $1")
                .bind(updated.id)
                .execute(pool)
                .await?;
        }

        Ok(updated)
    }

    pub async fn process_exit(
        &self,
        pool: &PgPool,
        access_code: &str,
        barrier_id: Option<Uuid>,
    ) -> AppResult<GuestAccess> {
        let guest_access = sqlx::query_as::<_, GuestAccess>(
            r#"
            SELECT * FROM guest_access
            WHERE access_code = $1 AND status = 'active'
            "#,
        )
        .bind(access_code)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Активный гостевой доступ не найден".to_string()))?;

        // Обновить статус
        let updated = sqlx::query_as::<_, GuestAccess>(
            r#"
            UPDATE guest_access
            SET status = 'completed', exited_at = NOW()
            WHERE id = $1
            RETURNING *
            "#,
        )
        .bind(guest_access.id)
        .fetch_one(pool)
        .await?;

        // Записать лог
        sqlx::query(
            r#"
            INSERT INTO barrier_access_logs
                (complex_id, barrier_id, guest_access_id, action, vehicle_number)
            VALUES ($1, $2, $3, $4, $5)
            "#,
        )
        .bind(guest_access.complex_id)
        .bind(barrier_id)
        .bind(guest_access.id)
        .bind(BarrierAction::Exit)
        .bind(&guest_access.vehicle_number)
        .execute(pool)
        .await?;

        Ok(updated)
    }

    pub async fn cancel_access(&self, pool: &PgPool, access_id: Uuid, user_id: Uuid) -> AppResult<()> {
        let result = sqlx::query(
            r#"
            UPDATE guest_access
            SET status = 'cancelled'
            WHERE id = $1 AND created_by = $2 AND status = 'pending'
            "#,
        )
        .bind(access_id)
        .bind(user_id)
        .execute(pool)
        .await?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Гостевой доступ не найден".to_string()));
        }

        Ok(())
    }

    pub async fn check_overstays(&self, pool: &PgPool) -> AppResult<()> {
        // Найти гостей, которые превысили время
        let overstays = sqlx::query_as::<_, GuestAccess>(
            r#"
            SELECT * FROM guest_access
            WHERE status = 'active'
              AND overstay_notified = false
              AND entered_at + (duration_minutes || ' minutes')::interval < NOW()
            "#,
        )
        .fetch_all(pool)
        .await?;

        for access in overstays {
            if let Some(owner_phone) = self.get_owner_phone(pool, access.created_by).await? {
                let guest_name = access.guest_name.clone().unwrap_or_else(|| "Гость".to_string());

                if let Err(e) = self.sms_service.send_overstay_notification(
                    &owner_phone,
                    &guest_name,
                    access.duration_minutes
                ).await {
                    tracing::error!("Failed to send overstay notification: {}", e);
                }

                sqlx::query("UPDATE guest_access SET overstay_notified = true WHERE id = $1")
                    .bind(access.id)
                    .execute(pool)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn expire_old_access(&self, pool: &PgPool) -> AppResult<i64> {
        let result = sqlx::query(
            r#"
            UPDATE guest_access
            SET status = 'expired'
            WHERE status = 'pending' AND expires_at < NOW()
            "#,
        )
        .execute(pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    async fn get_owner_phone(&self, pool: &PgPool, user_id: Uuid) -> AppResult<Option<String>> {
        let result = sqlx::query_as::<_, (String,)>(
            "SELECT phone FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(pool)
        .await?;

        Ok(result.map(|(phone,)| phone))
    }

    pub async fn get_active_guests(pool: &PgPool, complex_id: Uuid) -> AppResult<Vec<GuestAccess>> {
        let guests = sqlx::query_as::<_, GuestAccess>(
            r#"
            SELECT * FROM guest_access
            WHERE complex_id = $1 AND status IN ('pending', 'active')
            ORDER BY created_at DESC
            "#,
        )
        .bind(complex_id)
        .fetch_all(pool)
        .await?;

        Ok(guests)
    }

    pub async fn get_access_history(
        pool: &PgPool,
        complex_id: Uuid,
        limit: i64,
        offset: i64,
    ) -> AppResult<Vec<GuestAccess>> {
        let history = sqlx::query_as::<_, GuestAccess>(
            r#"
            SELECT * FROM guest_access
            WHERE complex_id = $1
            ORDER BY created_at DESC
            LIMIT $2 OFFSET $3
            "#,
        )
        .bind(complex_id)
        .bind(limit)
        .bind(offset)
        .fetch_all(pool)
        .await?;

        Ok(history)
    }
}

// QR код генерация
pub fn generate_qr_code(data: &str) -> AppResult<Vec<u8>> {
    use qrcode::QrCode;
    use image::Luma;

    let code = QrCode::new(data.as_bytes())
        .map_err(|e| AppError::Internal(e.to_string()))?;

    let image = code.render::<Luma<u8>>().build();

    let mut buffer = std::io::Cursor::new(Vec::new());
    image.write_to(&mut buffer, image::ImageFormat::Png)
        .map_err(|e| AppError::Internal(e.to_string()))?;

    Ok(buffer.into_inner())
}

pub fn generate_qr_code_base64(data: &str) -> AppResult<String> {
    let png_data = generate_qr_code(data)?;
    Ok(format!("data:image/png;base64,{}", base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &png_data)))
}
