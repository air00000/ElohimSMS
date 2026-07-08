use crate::{config::Config, sms::SmsClient};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub sms_client: Arc<SmsClient>,
    pub api_key: Option<String>,
    pub internal_bot_token: String,
    pub bot_internal_url: String,
    pub http_client: reqwest::Client,
}

impl AppState {
    pub fn new(pool: PgPool, config: &Config, sms_client: Arc<SmsClient>) -> Self {
        Self {
            pool,
            sms_client,
            api_key: config.api_key.clone(),
            internal_bot_token: config.internal_bot_token.clone(),
            bot_internal_url: config.bot_internal_url.clone(),
            http_client: reqwest::Client::new(),
        }
    }
}
