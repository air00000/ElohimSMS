use crate::{captcha::CaptchaVerifier, config::Config, sms::SmsClient};
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub sms_client: Arc<SmsClient>,
    pub captcha_verifier: Arc<CaptchaVerifier>,
    pub internal_bot_token: String,
    pub captcha_site_url: String,
}

impl AppState {
    pub fn new(pool: PgPool, config: &Config, sms_client: Arc<SmsClient>) -> Self {
        Self {
            pool,
            sms_client,
            captcha_verifier: Arc::new(CaptchaVerifier::new(config)),
            internal_bot_token: config.internal_bot_token.clone(),
            captcha_site_url: config.captcha_site_url.clone(),
        }
    }
}
