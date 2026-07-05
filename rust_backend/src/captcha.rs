use crate::{config::Config, error::AppError};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct RecaptchaResponse {
    success: bool,
    #[serde(rename = "error-codes")]
    error_codes: Option<Vec<String>>,
}

pub struct CaptchaVerifier {
    secret: String,
    client: reqwest::Client,
}

impl CaptchaVerifier {
    pub fn new(config: &Config) -> Self {
        Self {
            secret: config.recaptcha_secret.clone(),
            client: reqwest::Client::new(),
        }
    }

    pub async fn verify(&self, response: &str) -> Result<(), AppError> {
        if self.secret.is_empty() {
            // Если секрет не настроен, пропускаем проверку (режим разработки)
            tracing::warn!("RECAPTCHA_SECRET not set, skipping captcha verification");
            return Ok(());
        }

        if response.is_empty() {
            return Err(AppError::BadRequest("Captcha response is required".to_string()));
        }

        let form = [
            ("secret", self.secret.as_str()),
            ("response", response),
        ];

        let result = self
            .client
            .post("https://www.google.com/recaptcha/api/siteverify")
            .form(&form)
            .send()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to verify captcha: {e}")))?
            .json::<RecaptchaResponse>()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to parse captcha response: {e}")))?;

        if result.success {
            Ok(())
        } else {
            tracing::warn!("reCAPTCHA verification failed: {:?}", result.error_codes);
            Err(AppError::Forbidden)
        }
    }
}
