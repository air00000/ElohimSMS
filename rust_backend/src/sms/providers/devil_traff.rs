use crate::sms::{SmsProvider, SmsResult};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::{json, Value};
use tracing::{debug, instrument};

/// Конфигурация для одного аккаунта/эндпоинта Devil-Traff.
#[derive(Debug, Clone)]
pub struct DevilTraffConfig {
    pub base_url: String,
    pub auth_token: String,
    pub route: String,
    pub sender_id: String,
}

#[derive(Debug, Clone)]
pub struct DevilTraffProvider {
    name: String,
    client: Client,
    config: DevilTraffConfig,
}

impl DevilTraffProvider {
    pub fn new(name: impl Into<String>, config: DevilTraffConfig) -> Self {
        Self {
            name: name.into(),
            client: Client::new(),
            config,
        }
    }
}

#[async_trait]
impl SmsProvider for DevilTraffProvider {
    fn name(&self) -> &str {
        &self.name
    }

    /// Отправляет одно SMS через Devil-Traff API.
    ///
    /// Документация: PUT /api/send-sms
    /// Body: { route, sender_id, number, message }
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
                    "message": "SMS gateway URL is not configured. Message logged but not sent.",
                    "phone": phone,
                }),
            ));
        }

        let url = format!("{}/api/send-sms", self.config.base_url);
        let sender = sender_id
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.config.sender_id);

        let body = json!({
            "route": self.config.route,
            "sender_id": sender,
            "number": phone,
            "message": message,
        });

        debug!(
            url = %url,
            route = %self.config.route,
            sender_id = %sender,
            provider = %self.name,
            "Sending SMS via Devil-Traff"
        );

        let response = self
            .client
            .request(Method::PUT, &url)
            .bearer_auth(&self.config.auth_token)
            .json(&body)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to send request to Devil-Traff provider {}",
                    self.name
                )
            })?;

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
                "Devil-Traff SMS gateway returned error status"
            );
        }

        Ok(SmsResult::new(&self.name, success, provider_response))
    }

    /// Запрашивает баланс у SMS-шлюза.
    async fn get_balance(&self) -> anyhow::Result<Value> {
        if self.config.base_url.is_empty() || self.config.base_url.contains("example.com") {
            return Ok(json!({
                "mock": true,
                "balance": "N/A",
                "message": "SMS gateway URL is not configured",
            }));
        }

        let url = format!("{}/api/balance", self.config.base_url);
        let response = self
            .client
            .request(Method::GET, &url)
            .bearer_auth(&self.config.auth_token)
            .send()
            .await
            .with_context(|| format!("Failed to request balance for provider {}", self.name))?;

        let status = response.status();
        let body = response
            .json::<Value>()
            .await
            .unwrap_or_else(|_| json!({ "raw": "non-json response" }));

        if !status.is_success() {
            tracing::warn!(
                status = %status,
                response = %body,
                provider = %self.name,
                "Devil-Traff balance request returned error"
            );
        }

        Ok(body)
    }
}
