use super::config::SmsGatewayConfig;
use anyhow::Context;
use reqwest::{Client, Method};
use serde_json::{json, Value};
use tracing::{debug, instrument};

#[derive(Debug, Clone)]
pub struct SmsClient {
    client: Client,
    config: SmsGatewayConfig,
}

#[derive(Debug, Clone)]
pub struct SmsResult {
    pub success: bool,
    pub provider_response: Value,
}

impl SmsClient {
    pub fn new(config: SmsGatewayConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Отправляет одно SMS через Devil-Traff API.
    ///
    /// Документация: PUT /api/send-sms
    /// Body: { route, sender_id, number, message }
    #[instrument(skip(self), fields(phone = %phone))]
    pub async fn send_sms(
        &self,
        phone: &str,
        message: &str,
        sender_id: Option<&str>,
    ) -> anyhow::Result<SmsResult> {
        if self.config.base_url.is_empty() || self.config.base_url.contains("example.com") {
            return Ok(SmsResult {
                success: true,
                provider_response: json!({
                    "mock": true,
                    "message": "SMS gateway URL is not configured. Message logged but not sent.",
                    "phone": phone,
                }),
            });
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
            "Sending SMS via Devil-Traff"
        );

        let response = self
            .client
            .request(Method::PUT, &url)
            .bearer_auth(&self.config.auth_token)
            .json(&body)
            .send()
            .await
            .context("Failed to send request to Devil-Traff SMS gateway")?;

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
                "Devil-Traff SMS gateway returned error status"
            );
        }

        Ok(SmsResult {
            success,
            provider_response,
        })
    }

    /// Запрашивает баланс у SMS-шлюза.
    pub async fn get_balance(&self) -> anyhow::Result<serde_json::Value> {
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
            .context("Failed to request SMS gateway balance")?;

        let status = response.status();
        let body = response
            .json::<Value>()
            .await
            .unwrap_or_else(|_| json!({ "raw": "non-json response" }));

        if !status.is_success() {
            tracing::warn!(
                status = %status,
                response = %body,
                "SMS gateway balance request returned error"
            );
        }

        Ok(body)
    }
}
