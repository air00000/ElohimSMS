use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct SmsLog {
    pub id: Uuid,
    pub phone: String,
    pub message: String,
    pub status: String,
    pub provider_response: Option<serde_json::Value>,
    pub provider_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendSmsRequest {
    /// Номер получателя в международном формате (E.164).
    #[schema(example = "+79991234567")]
    pub phone: String,

    /// Целевая ссылка. Если для страны получателя есть активный шаблон,
    /// ссылка будет обёрнута в короткую и подставлена в текст шаблона.
    /// Иначе значение отправляется как текст SMS.
    #[schema(example = "https://example.com/landing")]
    pub message: String,

    /// Альфавитное имя отправителя (sender ID). По умолчанию — `TRACKING`.
    #[schema(example = "MYBRAND")]
    pub sender_id: Option<String>,

    /// Идентификатор сущности во внешней системе.
    ///
    /// Например: Telegram chat_id, order_id или UUID операции.
    /// Возвращается без изменений в webhook-событии.
    #[schema(example = "order-12345")]
    pub external_id: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendSmsResponse {
    /// true, если SMS-шлюз принял сообщение.
    #[schema(example = true)]
    pub success: bool,

    /// Человекочитаемый статус отправки.
    #[schema(example = "Campaign sent via API")]
    pub message: String,

    /// Сырой ответ SMS-провайдера (для отладки).
    pub provider_response: Option<serde_json::Value>,

    /// Идентификатор кампании, если ссылка была обёрнута в короткую.
    pub campaign_id: Option<Uuid>,

    /// Короткая ссылка, отправленная получателю (если применялся шаблон).
    #[schema(example = "https://linkre.info/r/aB3dE5fG")]
    pub short_link: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    /// Общий статус сервиса.
    #[schema(example = "ok")]
    pub status: String,

    /// Доступность базы данных: `ok` или `error`.
    #[schema(example = "ok")]
    pub database: String,

    /// Время ответа в формате RFC 3339.
    #[schema(example = "2026-07-20T12:00:00Z")]
    pub timestamp: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Admin {
    pub id: Uuid,
    pub telegram_id: i64,
    pub username: Option<String>,
    pub is_owner: bool,
    pub sender_name: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAdminRequest {
    pub telegram_id: i64,
    pub username: Option<String>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct ApiKey {
    pub id: Uuid,
    pub key_hash: String,
    pub name: String,
    pub is_active: bool,
    pub created_by_telegram_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct ApiKeyListItem {
    pub id: Uuid,
    pub name: String,
    pub is_active: bool,
    pub created_by_telegram_id: Option<i64>,
    pub created_at: DateTime<Utc>,
    pub last_used_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateKeyRequest {
    pub name: String,
    pub created_by_telegram_id: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateKeyResponse {
    pub id: Uuid,
    pub name: String,
    pub key: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Template {
    pub id: Uuid,
    pub country_code: String,
    pub name: Option<String>,
    pub text: String,
    pub is_active: bool,
    pub is_favorite: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTemplateRequest {
    pub country_code: String,
    pub name: String,
    pub text: String,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Campaign {
    pub id: Uuid,
    pub short_code: String,
    pub target_url: String,
    pub phone: String,
    pub country_code: String,
    pub message: String,
    pub template_name: Option<String>,
    pub status: String,
    pub click_count: i32,
    pub sent_by_telegram_id: Option<i64>,
    pub api_key_id: Option<Uuid>,
    pub external_id: Option<String>,
    pub provider_response: Option<serde_json::Value>,
    pub provider_name: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
    pub first_clicked_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BotSendSmsRequest {
    pub phone: String,
    pub message: String,
    pub telegram_id: i64,
    pub url: Option<String>,
    pub template_name: Option<String>,
    pub sender_id: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSenderNameRequest {
    pub telegram_id: i64,
    pub sender_name: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct EnsureOwnerRequest {
    pub telegram_id: i64,
    pub username: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct StatsResponse {
    pub admins_count: i64,
    pub keys_total: i64,
    pub keys_active: i64,
    pub balance: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ConfigureWebhookRequest {
    /// Публичный HTTPS endpoint, который будет принимать события.
    #[schema(example = "https://example.com/webhooks/elohim")]
    pub url: String,

    /// Секрет для проверки подписи HMAC-SHA256. Минимум 16 символов.
    #[schema(example = "my-super-secret-key-32chars")]
    pub secret: String,

    /// Включить или выключить webhook. По умолчанию — `true`.
    #[schema(example = true)]
    pub is_active: Option<bool>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ConfigureWebhookResponse {
    /// true, если настройки сохранены.
    #[schema(example = true)]
    pub success: bool,

    /// Сохранённый URL webhook.
    #[schema(example = "https://example.com/webhooks/elohim")]
    pub url: String,

    /// Активен ли webhook после изменения.
    #[schema(example = true)]
    pub is_active: bool,
}
