use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_address: SocketAddr,
    pub internal_bot_token: String,
    pub sms_gateway_url: String,
    pub sms_gateway_auth_token: String,
    pub sms_gateway_route: String,
    pub sms_gateway_sender_id: String,
    pub captcha_site_url: String,
    pub recaptcha_secret: String,
    #[allow(dead_code)]
    pub recaptcha_site_key: String,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;

        let bind_address = std::env::var("BIND_ADDRESS")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
            .parse()?;

        let internal_bot_token = std::env::var("INTERNAL_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("INTERNAL_BOT_TOKEN must be set"))?;

        let sms_gateway_url = std::env::var("SMS_GATEWAY_URL")
            .unwrap_or_else(|_| "https://api.devil-traff.cc".to_string())
            .trim_end_matches('/')
            .to_string();

        let sms_gateway_auth_token = std::env::var("SMS_GATEWAY_AUTH_TOKEN")
            .map_err(|_| anyhow::anyhow!("SMS_GATEWAY_AUTH_TOKEN must be set"))?;

        let sms_gateway_route = std::env::var("SMS_GATEWAY_ROUTE")
            .unwrap_or_else(|_| "Auto".to_string());

        let sms_gateway_sender_id = std::env::var("SMS_GATEWAY_SENDER_ID")
            .unwrap_or_else(|_| "ElohimSMS".to_string());

        let captcha_site_url = std::env::var("CAPTCHA_SITE_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string())
            .trim_end_matches('/')
            .to_string();

        let recaptcha_secret = std::env::var("RECAPTCHA_SECRET").unwrap_or_default();
        let recaptcha_site_key = std::env::var("RECAPTCHA_SITE_KEY").unwrap_or_default();

        Ok(Self {
            database_url,
            bind_address,
            internal_bot_token,
            sms_gateway_url,
            sms_gateway_auth_token,
            sms_gateway_route,
            sms_gateway_sender_id,
            captcha_site_url,
            recaptcha_secret,
            recaptcha_site_key,
        })
    }
}
