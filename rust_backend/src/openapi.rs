use crate::models::{
    Admin, ApiKeyListItem, BotSendSmsRequest, Campaign, CreateAdminRequest, CreateKeyRequest,
    CreateKeyResponse, CreateTemplateRequest, EnsureOwnerRequest, HealthResponse,
    SendCampaignRequest, SendCampaignResponse, SendSmsRequest, SendSmsResponse, SmsLog,
    StatsResponse, Template, UpdateSenderNameRequest,
};
use utoipa::openapi::security::{ApiKey as SecurityApiKey, ApiKeyValue, SecurityScheme};
use utoipa::{Modify, OpenApi};

pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "api_key",
                SecurityScheme::ApiKey(SecurityApiKey::Header(ApiKeyValue::new("X-API-Key"))),
            );
            components.add_security_scheme(
                "internal_bot_token",
                SecurityScheme::ApiKey(SecurityApiKey::Header(ApiKeyValue::new(
                    "X-Internal-Bot-Token",
                ))),
            );
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::health,
        crate::routes::sms::send_sms,
        crate::routes::bot::list_admins,
        crate::routes::bot::create_admin,
        crate::routes::bot::ensure_owner,
        crate::routes::bot::update_sender_name,
        crate::routes::bot::remove_admin,
        crate::routes::bot::list_keys,
        crate::routes::bot::create_key,
        crate::routes::bot::revoke_key,
        crate::routes::bot::list_templates,
        crate::routes::bot::create_template,
        crate::routes::bot::delete_template,
        crate::routes::bot::set_favorite_template,
        crate::routes::bot::bot_send_sms,
        crate::routes::bot::send_campaign,
        crate::routes::bot::stats,
    ),
    components(
        schemas(
            SmsLog,
            SendSmsRequest,
            SendSmsResponse,
            HealthResponse,
            Admin,
            CreateAdminRequest,
            ApiKeyListItem,
            CreateKeyRequest,
            CreateKeyResponse,
            Template,
            CreateTemplateRequest,
            Campaign,
            SendCampaignRequest,
            SendCampaignResponse,
            BotSendSmsRequest,
            UpdateSenderNameRequest,
            EnsureOwnerRequest,
            StatsResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "health", description = "Health check"),
        (name = "sms", description = "SMS sending"),
        (name = "bot", description = "Telegram bot internal API"),
    ),
    security(
        ("api_key" = [])
    )
)]
pub struct ApiDoc;
