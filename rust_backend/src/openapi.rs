use crate::models::{
    Admin, ApiKey, Campaign, CreateAdminRequest, CreateApiKeyRequest, CreateApiKeyResponse,
    CreateTemplateRequest, HealthResponse, LinkInfoResponse, SendCampaignRequest,
    SendCampaignResponse, SendSmsRequest, SendSmsResponse, SmsLog, Template,
    VerifyCaptchaRequest, VerifyCaptchaResponse,
};
use crate::pagination::PaginatedResponse;
use utoipa::OpenApi;

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::routes::health::health,
        crate::routes::admin::list_admins,
        crate::routes::admin::create_admin,
        crate::routes::admin::remove_admin,
        crate::routes::keys::list_keys,
        crate::routes::keys::create_key,
        crate::routes::keys::revoke_key,
        crate::routes::sms::send_sms,
        crate::routes::sms::get_balance,
        crate::routes::sms::get_routes,
        crate::routes::templates::list,
        crate::routes::templates::create_or_update,
        crate::routes::templates::remove,
        crate::routes::campaigns::send_campaign,
        crate::routes::campaigns::send_campaign_as_bot,
        crate::routes::links::get_link_info,
        crate::routes::links::verify_captcha,
    ),
    components(
        schemas(
            Admin,
            ApiKey,
            Campaign,
            SmsLog,
            Template,
            CreateAdminRequest,
            CreateApiKeyRequest,
            CreateApiKeyResponse,
            CreateTemplateRequest,
            SendCampaignRequest,
            SendCampaignResponse,
            SendSmsRequest,
            SendSmsResponse,
            HealthResponse,
            LinkInfoResponse,
            VerifyCaptchaRequest,
            VerifyCaptchaResponse,
            PaginatedResponse<Admin>,
            PaginatedResponse<ApiKey>,
        )
    ),
    tags(
        (name = "health", description = "Health check"),
        (name = "admin", description = "Admin management"),
        (name = "keys", description = "API key management"),
        (name = "sms", description = "SMS sending"),
        (name = "templates", description = "SMS templates by country"),
        (name = "campaigns", description = "Phishing campaigns"),
        (name = "links", description = "Short link verification"),
    ),
    security(
        ("api_key" = []),
        ("internal_bot_token" = [])
    )
)]
pub struct ApiDoc;
