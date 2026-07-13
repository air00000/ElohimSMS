use crate::sms::{SmsProvider, SmsResult};
use anyhow::Context;
use async_trait::async_trait;
use reqwest::{Client, Method};
use serde_json::{json, Value};
use tracing::{debug, instrument};

/// Конфигурация для аккаунта SMSMobile.cc.
#[derive(Debug, Clone)]
pub struct SmsMobileCcConfig {
    pub base_url: String,
    pub username: String,
    pub password: String,
    pub route_id: Option<String>,
    pub sender_id: String,
}

#[derive(Debug, Clone)]
pub struct SmsMobileCcProvider {
    name: String,
    client: Client,
    config: SmsMobileCcConfig,
}

impl SmsMobileCcProvider {
    pub fn new(name: impl Into<String>, config: SmsMobileCcConfig) -> Self {
        Self {
            name: name.into(),
            client: Client::new(),
            config,
        }
    }

    fn is_mock_url(&self) -> bool {
        self.config.base_url.is_empty()
            || self.config.base_url.contains("example.com")
            || self.config.username.is_empty()
            || self.config.password.is_empty()
    }
}

#[async_trait]
impl SmsProvider for SmsMobileCcProvider {
    fn name(&self) -> &str {
        &self.name
    }

    /// Отправляет SMS через SMSMobile.cc API.
    ///
    /// Документация: POST /api/sendsms
    /// Body: { username, password, type, source, destinations, message, route_id? }
    ///
    /// Важно передать `Accept: application/json`, иначе ошибки валидации придут
    /// как HTTP-редиректы.
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
                    "message": "SMSMobile.cc credentials are not configured. Message logged but not sent.",
                    "phone": phone,
                }),
            ));
        }

        let url = format!("{}/api/sendsms", self.config.base_url);
        let sender = sender_id
            .filter(|s| !s.is_empty())
            .unwrap_or(&self.config.sender_id);

        let mut body = json!({
            "username": self.config.username,
            "password": self.config.password,
            "type": 0,
            "source": sender,
            "destinations": phone,
            "message": message,
        });

        if let Some(route_id) = &self.config.route_id {
            if !route_id.is_empty() {
                body["route_id"] = json!(route_id);
            }
        }

        debug!(
            url = %url,
            sender_id = %sender,
            provider = %self.name,
            "Sending SMS via SMSMobile.cc"
        );

        let response = self
            .client
            .request(Method::POST, &url)
            .header("Accept", "application/json")
            .json(&body)
            .send()
            .await
            .with_context(|| {
                format!(
                    "Failed to send request to SMSMobile.cc provider {}",
                    self.name
                )
            })?;

        let status = response.status();
        let provider_response = response
            .json::<Value>()
            .await
            .unwrap_or_else(|_| json!({ "raw": "non-json response" }));

        let success = if status.is_success() {
            // В документации top-level status может быть delivered / partial / failed.
            // Для одного получателя считаем успехом только "delivered".
            provider_response
                .get("status")
                .and_then(|s| s.as_str())
                .map(|s| s.eq_ignore_ascii_case("delivered"))
                .unwrap_or(true)
        } else {
            false
        };

        if !success {
            tracing::warn!(
                status = %status,
                response = %provider_response,
                provider = %self.name,
                "SMSMobile.cc SMS gateway returned error status"
            );
        }

        Ok(SmsResult::new(&self.name, success, provider_response))
    }

    /// Запрашивает баланс аккаунта SMSMobile.cc.
    ///
    /// Документация: POST /api/get-balance
    /// Body: { username, password }
    async fn get_balance(&self) -> anyhow::Result<Value> {
        if self.is_mock_url() {
            return Ok(json!({
                "mock": true,
                "balance": "N/A",
                "message": "SMSMobile.cc credentials are not configured",
            }));
        }

        let url = format!("{}/api/get-balance", self.config.base_url);
        let body = json!({
            "username": self.config.username,
            "password": self.config.password,
        });

        let response = self
            .client
            .request(Method::POST, &url)
            .header("Accept", "application/json")
            .json(&body)
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
                "SMSMobile.cc balance request returned error"
            );
        }

        Ok(body)
    }
}
