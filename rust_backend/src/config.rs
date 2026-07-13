use crate::sms::{load_provider_configs_from_env, SmsProviderConfig};
use std::net::SocketAddr;

#[derive(Debug, Clone)]
pub struct Config {
    pub database_url: String,
    pub bind_address: SocketAddr,
    pub api_key: Option<String>,
    pub internal_bot_token: String,
    pub bot_internal_url: String,
    pub short_link_base_url: Option<String>,
    pub captcha_site_url: String,
    pub recaptcha_secret: String,
    pub link_path_prefix: String,
    pub sms_providers: Vec<SmsProviderConfig>,
}

impl Config {
    /// Формирует короткую ссылку для кампании.
    ///
    /// Приоритет:
    /// 1. `SHORT_LINK_BASE_URL` (если задан) — используется как есть + `/l/{short_code}`.
    /// 2. `CAPTCHA_SITE_URL` + `LINK_PATH_PREFIX` (по умолчанию `/l/{short_code}`).
    pub fn short_link(&self, short_code: &str) -> String {
        let base = self
            .short_link_base_url
            .as_deref()
            .unwrap_or(&self.captcha_site_url);
        format!("{}/{}/{}", base, self.link_path_prefix, short_code)
    }

    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;

        let bind_address = std::env::var("BIND_ADDRESS")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
            .parse()?;

        let api_key = std::env::var("API_KEY")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());

        let internal_bot_token = std::env::var("INTERNAL_BOT_TOKEN")
            .map_err(|_| anyhow::anyhow!("INTERNAL_BOT_TOKEN must be set"))?;

        let bot_internal_url = std::env::var("BOT_INTERNAL_URL")
            .unwrap_or_else(|_| "http://localhost:8080".to_string())
            .trim_end_matches('/')
            .to_string();

        let short_link_base_url = std::env::var("SHORT_LINK_BASE_URL")
            .ok()
            .map(|s| s.trim_end_matches('/').to_string())
            .filter(|s| !s.is_empty());

        let captcha_site_url = std::env::var("CAPTCHA_SITE_URL")
            .unwrap_or_else(|_| "https://linkre.info".to_string())
            .trim_end_matches('/')
            .to_string();

        let recaptcha_secret = std::env::var("RECAPTCHA_SECRET").unwrap_or_default();

        let link_path_prefix = std::env::var("LINK_PATH_PREFIX")
            .unwrap_or_else(|_| "l".to_string())
            .trim_matches('/')
            .to_string();

        let sms_providers = load_provider_configs_from_env();

        Ok(Self {
            database_url,
            bind_address,
            api_key,
            internal_bot_token,
            bot_internal_url,
            short_link_base_url,
            captcha_site_url,
            recaptcha_secret,
            link_path_prefix,
            sms_providers,
        })
    }
}
