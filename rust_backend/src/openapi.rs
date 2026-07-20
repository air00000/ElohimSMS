use crate::models::{
    ConfigureWebhookRequest, ConfigureWebhookResponse, HealthResponse, SendSmsRequest,
    SendSmsResponse,
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
        }
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "ElohimSMS API",
        description = "Публичный API для отправки SMS с отслеживаемыми ссылками.\n\n\
            Аутентификация: передайте ваш ключ в заголовке `X-API-Key`.\n\n\
            Когда получатель переходит по ссылке из SMS, на ваш webhook \
            отправляется событие `campaign.link_verified`, подписанное HMAC-SHA256 \
            (заголовок `X-Elohim-Signature`)."
    ),
    paths(
        crate::routes::health::health,
        crate::routes::sms::send_sms,
        crate::routes::webhooks::configure_webhook,
    ),
    components(
        schemas(
            SendSmsRequest,
            SendSmsResponse,
            HealthResponse,
            ConfigureWebhookRequest,
            ConfigureWebhookResponse,
        )
    ),
    modifiers(&SecurityAddon),
    tags(
        (name = "sms", description = "Отправка SMS"),
        (name = "webhook", description = "Настройка webhook для событий"),
        (name = "health", description = "Состояние сервиса"),
    ),
    security(
        ("api_key" = [])
    )
)]
pub struct ApiDoc;
