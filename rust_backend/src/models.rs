use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Admin {
    pub id: Uuid,
    pub telegram_id: i64,
    pub username: Option<String>,
    pub is_owner: bool,
    pub created_at: DateTime<Utc>,
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
pub struct SmsLog {
    pub id: Uuid,
    pub api_key_id: Option<Uuid>,
    pub campaign_id: Option<Uuid>,
    pub telegram_id: Option<i64>,
    pub phone: String,
    pub message: String,
    pub status: String,
    pub provider_response: Option<serde_json::Value>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Template {
    pub country_code: String,
    pub text: String,
    pub is_active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, FromRow, ToSchema)]
pub struct Campaign {
    pub id: Uuid,
    pub short_code: String,
    pub target_url: String,
    pub phone: String,
    pub country_code: String,
    pub message: String,
    pub status: String,
    pub click_count: i32,
    pub verified_count: i32,
    pub provider_response: Option<serde_json::Value>,
    pub sent_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateAdminRequest {
    pub telegram_id: i64,
    pub username: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateApiKeyRequest {
    pub name: String,
    pub created_by_telegram_id: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CreateApiKeyResponse {
    pub id: Uuid,
    pub key: String,
    pub name: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendSmsRequest {
    pub phone: String,
    pub message: String,
    pub telegram_id: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendSmsResponse {
    pub success: bool,
    pub message: String,
    pub provider_response: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub database: String,
    pub timestamp: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateTemplateRequest {
    pub country_code: String,
    pub text: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct SendCampaignRequest {
    pub phone: String,
    pub url: String,
    pub telegram_id: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct SendCampaignResponse {
    pub success: bool,
    pub campaign_id: Uuid,
    pub short_link: String,
    pub message: String,
    pub provider_response: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct LinkInfoResponse {
    pub short_code: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct VerifyCaptchaRequest {
    pub g_recaptcha_response: String,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct VerifyCaptchaResponse {
    pub target_url: String,
}
