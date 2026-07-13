use crate::sms::{SmsProvider, SmsResult};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::{json, Value};
use tracing::{debug, instrument};

/// Конфигурация SkyTelecom.
#[derive(Debug, Clone)]
pub struct SkyTelecomConfig {
    pub base_url: String,
    pub api_key: String,
}

#[derive(Debug, Clone)]
pub struct SkyTelecomProvider {
    name: String,
    client: Client,
    config: SkyTelecomConfig,
}

impl SkyTelecomProvider {
    pub fn new(name: impl Into<String>, config: SkyTelecomConfig) -> Self {
        Self {
            name: name.into(),
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl SmsProvider for SkyTelecomProvider {
    fn name(&self) -> &str {
        &self.name
    }

    /// Отправляет SMS через SkyTelecom API.
    ///
    /// Endpoint: POST /api/sms/send
    /// Body: { to, message, from }
    /// Auth: Authorization: Bearer <api_key>
    #[instrument(skip(self), fields(phone = %phone, provider = %self.name))]
    async fn send_sms(
        &self,
        phone: &str,
        message: &str,
        sender_id: Option<&str>,
    ) -> anyhow::Result<SmsResult> {
        if self.config.base_url.is_empty() || self.config.base_url.contains("example.com") {
            return Ok(SmsResult::new(
                &self.name,
                true,
                json!({
                    "mock": true,
                    "message": "SkyTelecom URL is not configured. Message logged but not sent.",
                    "phone": phone,
                }),
            ));
        }

        let url = format!("{}/api/sms/send", self.config.base_url);
        let sender = sender_id.unwrap_or("ElohimSMS");

        let body = json!({
            "to": phone,
            "message": message,
            "from": sender,
        });

        debug!(
            url = %url,
            sender_id = %sender,
            provider = %self.name,
            "Sending SMS via SkyTelecom"
        );

        let response = self
            .client
            .request(Method::POST, &url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .with_context(|| format!("Failed to send request to SkyTelecom provider {}", self.name))?;

        let status = response.status();
        let provider_response = response
            .json::<Value>()
            .await
            .unwrap_or_else(|_| json!({ "raw": "non-json response" }));

        let success = status.is_success();

        if !success {
            tracing::warn!(
                status = %status,
                response = %provider_response,
                provider = %self.name,
                "SkyTelecom SMS gateway returned error status"
            );
        }

        Ok(SmsResult::new(&self.name, success, provider_response))
    }

    /// SkyTelecom не предоставляет публичного endpoint баланса в открытой документации.
    async fn get_balance(&self) -> anyhow::Result<Value> {
        Ok(json!({
            "mock": true,
            "balance": "N/A",
            "message": "Balance check is not supported for SkyTelecom",
        }))
    }
}
