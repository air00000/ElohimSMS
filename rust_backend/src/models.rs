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
    pub phone: String,
    pub message: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendSmsResponse {
    pub success: bool,
    pub message: String,
    pub provider_response: Option<serde_json::Value>,
    pub campaign_id: Option<Uuid>,
    pub short_link: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
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
    pub provider_response: Option<serde_json::Value>,
    pub provider_name: Option<String>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendCampaignRequest {
    pub phone: String,
    pub url: String,
    pub telegram_id: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendCampaignResponse {
    pub success: bool,
    pub campaign_id: Uuid,
    pub short_link: String,
    pub message: String,
    pub provider_response: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct BotSendSmsRequest {
    pub phone: String,
    pub message: String,
    pub telegram_id: i64,
    pub url: Option<String>,
    pub template_name: Option<String>,
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
