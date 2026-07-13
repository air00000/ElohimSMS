use async_trait::async_trait;
use serde_json::Value;

/// Результат отправки SMS через конкретного провайдера.
#[derive(Debug, Clone)]
pub struct SmsResult {
    pub success: bool,
    pub provider_response: Value,
    pub provider_name: String,
}

impl SmsResult {
    pub fn new(provider_name: impl Into<String>, success: bool, provider_response: Value) -> Self {
        Self {
            success,
            provider_response,
            provider_name: provider_name.into(),
        }
    }

    /// Оборачивает сырой ответ провайдера в объект с указанием имени провайдера.
    pub fn provider_response_json(&self) -> Value {
        serde_json::json!({
            "provider": self.provider_name,
            "raw": self.provider_response,
        })
    }
}

/// Описание неудавшейся попытки отправки через провайдера.
#[derive(Debug, Clone)]
pub struct ProviderAttempt {
    pub provider_name: String,
    pub error: String,
    pub provider_response: Option<Value>,
}

/// Абстракция SMS-провайдера.
///
/// Новые сервисы добавляются реализацией этого трейта + регистрацией в фабрике
/// `SmsFailoverClient::from_configs`.
#[async_trait]
pub trait SmsProvider: Send + Sync + std::fmt::Debug {
    fn name(&self) -> &str;

    async fn send_sms(
        &self,
        phone: &str,
        message: &str,
        sender_id: Option<&str>,
    ) -> anyhow::Result<SmsResult>;

    async fn get_balance(&self) -> anyhow::Result<Value>;
}
