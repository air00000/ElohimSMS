use crate::sms::{ProviderAttempt, SmsProvider, SmsResult};
use anyhow::Context;
use serde_json::{json, Value};
use std::sync::Arc;

/// Клиент, который последовательно перебирает настроенных SMS-провайдеров
/// до первой успешной отправки.
#[derive(Clone)]
pub struct SmsFailoverClient {
    providers: Vec<Arc<dyn SmsProvider>>,
}

impl SmsFailoverClient {
    pub fn new(providers: Vec<Arc<dyn SmsProvider>>) -> Self {
        Self { providers }
    }

    /// Отправляет SMS, перебирая провайдеров по порядку.
    ///
    /// Возвращает результат первого успешного провайдера.
    /// Если все провайдеры отказали — возвращает ошибку с описанием всех попыток.
    pub async fn send_sms(
        &self,
        phone: &str,
        message: &str,
        sender_id: Option<&str>,
    ) -> anyhow::Result<SmsResult> {
        if self.providers.is_empty() {
            anyhow::bail!("No SMS providers configured");
        }

        let mut attempts = Vec::new();

        for provider in &self.providers {
            match provider.send_sms(phone, message, sender_id).await {
                Ok(result) if result.success => {
                    tracing::info!(
                        phone = %phone,
                        provider = %provider.name(),
                        "SMS sent successfully via provider"
                    );
                    return Ok(result);
                }
                Ok(result) => {
                    tracing::warn!(
                        phone = %phone,
                        provider = %provider.name(),
                        response = %result.provider_response,
                        "Provider rejected SMS, trying next provider"
                    );
                    attempts.push(ProviderAttempt {
                        provider_name: provider.name().to_string(),
                        error: "Provider returned unsuccessful response".to_string(),
                        provider_response: Some(result.provider_response),
                    });
                }
                Err(err) => {
                    tracing::warn!(
                        phone = %phone,
                        provider = %provider.name(),
                        error = %err,
                        "Provider failed, trying next provider"
                    );
                    attempts.push(ProviderAttempt {
                        provider_name: provider.name().to_string(),
                        error: err.to_string(),
                        provider_response: None,
                    });
                }
            }
        }

        let failed_providers: Vec<Value> = attempts
            .iter()
            .map(|a| {
                json!({
                    "provider": a.provider_name,
                    "error": a.error,
                    "response": a.provider_response,
                })
            })
            .collect();

        Err(anyhow::anyhow!(
            "All SMS providers failed. Attempts: {}",
            serde_json::to_string_pretty(&failed_providers).unwrap_or_default()
        ))
        .with_context(|| format!("Failed to send SMS to {}", phone))
    }

    /// Возвращает баланс первого доступного провайдера.
    ///
    /// Если ни один не ответил — возвращает агрегированный JSON с ошибками.
    pub async fn get_balance(&self) -> anyhow::Result<Value> {
        if self.providers.is_empty() {
            return Ok(json!({
                "error": "No SMS providers configured",
                "balance": "N/A",
            }));
        }

        let mut attempts = Vec::new();

        for provider in &self.providers {
            match provider.get_balance().await {
                Ok(balance) => {
                    return Ok(json!({
                        "provider": provider.name(),
                        "balance": balance,
                    }));
                }
                Err(err) => {
                    attempts.push(json!({
                        "provider": provider.name(),
                        "error": err.to_string(),
                    }));
                }
            }
        }

        Ok(json!({
            "error": "All providers failed to return balance",
            "attempts": attempts,
        }))
    }
}

impl std::fmt::Debug for SmsFailoverClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SmsFailoverClient")
            .field("providers", &self.providers.len())
            .finish()
    }
}
