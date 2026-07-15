use crate::sms::{SmsProvider, SmsResult};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::{json, Value};
use tracing::{debug, instrument};

/// Конфигурация для аккаунта LimitlessTXT.
///
/// API: https://api.limitlesstxt.com/swagger
/// Auth: Bearer <API key> (ключ обычно начинается с `ltxt_`).
#[derive(Debug, Clone)]
pub struct LimitlessTxtConfig {
    pub base_url: String,
    pub api_key: String,
    pub sender_id: String,
    pub route: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct LimitlessTxtProvider {
    name: String,
    client: Client,
    config: LimitlessTxtConfig,
}

impl LimitlessTxtProvider {
    pub fn new(name: impl Into<String>, config: LimitlessTxtConfig) -> Self {
        Self {
            name: name.into(),
            client: Client::new(),
            config,
        }
    }

    fn is_mock_url(&self) -> bool {
        self.config.base_url.is_empty()
            || self.config.base_url.contains("example.com")
            || self.config.api_key.is_empty()
    }
}

#[async_trait]
impl SmsProvider for LimitlessTxtProvider {
    fn name(&self) -> &str {
        &self.name
    }

    /// Отправляет SMS через LimitlessTXT Public API.
    ///
    /// Endpoint: POST /v1/send
    /// Body: { numbers, content, sender_id, route? }
    #[instrument(skip(self), fields(phone = %phone, provider = %self.name))]
    async fn send_sms(
        &self,
        phone: &str,
        message: &str,
        sender_id: Option<&str>,
    ) -> anyhow::Result<SmsResult> {
        if self.is_mock_url() {
            return Ok(SmsResult::new(
                &self.name,
                true,
                json!({
                    "mock": true,
                    "message": "LimitlessTXT credentials are not configured. Message logged but not sent.",
                    "phone": phone,
                }),
            ));
        }

        let url = format!("{}/v1/send", self.config.base_url);
        let sender = sender_id
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.config.sender_id);

        // Sender ID в LimitlessTXT ограничен 11 символами.
        let sender: String = sender.chars().take(11).collect();

        let mut body = json!({
            "numbers": phone,
            "content": message,
            "sender_id": sender,
        });

        if let Some(route) = self.config.route {
            body["route"] = json!(route);
        }

        debug!(
            url = %url,
            sender_id = %sender,
            route = ?self.config.route,
            provider = %self.name,
            "Sending SMS via LimitlessTXT"
        );

        let response = self
            .client
            .request(Method::POST, &url)
            .bearer_auth(&self.config.api_key)
            .json(&body)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to send request to LimitlessTXT provider {}",
                    self.name
                )
            })?;

        let status = response.status();
        let provider_response = response
            .json::<Value>()
            .await
            .unwrap_or_else(|_| json!({ "raw": "non-json response" }));

        let success = status.is_success()
            && provider_response
                .get("success")
                .and_then(|v| v.as_bool())
                .unwrap_or(true);

        if !success {
            tracing::warn!(
                status = %status,
                response = %provider_response,
                provider = %self.name,
                "LimitlessTXT SMS gateway returned error"
            );
        }

        Ok(SmsResult::new(&self.name, success, provider_response))
    }

    /// Запрашивает баланс аккаунта LimitlessTXT.
    ///
    /// Endpoint: GET /v1/balance
    async fn get_balance(&self) -> anyhow::Result<Value> {
        if self.is_mock_url() {
            return Ok(json!({
                "mock": true,
                "balance": "N/A",
                "message": "LimitlessTXT credentials are not configured",
            }));
        }

        let url = format!("{}/v1/balance", self.config.base_url);

        let response = self
            .client
            .request(Method::GET, &url)
            .bearer_auth(&self.config.api_key)
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
                "LimitlessTXT balance request returned error"
            );
        }

        Ok(body)
    }
}
