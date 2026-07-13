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
    pub sms_providers: Vec<SmsProviderConfig>,
}

impl Config {
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

        let sms_providers = load_provider_configs_from_env();

        Ok(Self {
            database_url,
            bind_address,
            api_key,
            internal_bot_token,
            bot_internal_url,
            short_link_base_url,
            sms_providers,
        })
    }
}
