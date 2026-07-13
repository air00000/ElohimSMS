use crate::sms::providers::{DevilTraffConfig, SkyTelecomConfig, SmsMobileCcConfig};

/// Тип провайдера, определяющий реализацию.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderType {
    DevilTraff,
    SkyTelecom,
    SmsMobileCc,
}

impl ProviderType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_ascii_lowercase().as_str() {
            "devil-traff" | "devil_traff" => Some(ProviderType::DevilTraff),
            "skytelecom" | "sky-telecom" | "sky_telecom" => Some(ProviderType::SkyTelecom),
            "smsmobile" | "smsmobilecc" | "smsmobile-cc" | "sms_mobile" | "sms_mobile_cc" => {
                Some(ProviderType::SmsMobileCc)
            }
            _ => None,
        }
    }
}

/// Конфигурация одного SMS-провайдера.
#[derive(Debug, Clone)]
pub struct SmsProviderConfig {
    pub name: String,
    pub provider_type: ProviderType,
    pub priority: i32,
    pub devil_traff: Option<DevilTraffConfig>,
    pub sky_telecom: Option<SkyTelecomConfig>,
    pub smsmobile_cc: Option<SmsMobileCcConfig>,
}

impl SmsProviderConfig {
    pub fn new_devil_traff(
        name: impl Into<String>,
        priority: i32,
        base_url: impl Into<String>,
        auth_token: impl Into<String>,
        route: impl Into<String>,
        sender_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provider_type: ProviderType::DevilTraff,
            priority,
            devil_traff: Some(DevilTraffConfig {
                base_url: base_url.into(),
                auth_token: auth_token.into(),
                route: route.into(),
                sender_id: sender_id.into(),
            }),
            sky_telecom: None,
            smsmobile_cc: None,
        }
    }

    pub fn new_sky_telecom(
        name: impl Into<String>,
        priority: i32,
        base_url: impl Into<String>,
        api_key: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provider_type: ProviderType::SkyTelecom,
            priority,
            devil_traff: None,
            sky_telecom: Some(SkyTelecomConfig {
                base_url: base_url.into(),
                api_key: api_key.into(),
            }),
            smsmobile_cc: None,
        }
    }

    pub fn new_smsmobile_cc(
        name: impl Into<String>,
        priority: i32,
        base_url: impl Into<String>,
        username: impl Into<String>,
        password: impl Into<String>,
        route_id: Option<String>,
        sender_id: impl Into<String>,
    ) -> Self {
        Self {
            name: name.into(),
            provider_type: ProviderType::SmsMobileCc,
            priority,
            devil_traff: None,
            sky_telecom: None,
            smsmobile_cc: Some(SmsMobileCcConfig {
                base_url: base_url.into(),
                username: username.into(),
                password: password.into(),
                route_id,
                sender_id: sender_id.into(),
            }),
        }
    }
}

/// Загружает список конфигураций провайдеров из переменных окружения.
///
/// Поддерживается два формата:
/// 1. Универсальный: SMS_PROVIDER_N_NAME, SMS_PROVIDER_N_TYPE, SMS_PROVIDER_N_URL,
///    SMS_PROVIDER_N_AUTH_TOKEN, SMS_PROVIDER_N_ROUTE, SMS_PROVIDER_N_SENDER_ID,
///    SMS_PROVIDER_N_PRIORITY (N = 1, 2, ...).
/// 2. Устаревший: SMS_GATEWAY_URL, SMS_GATEWAY_AUTH_TOKEN, SMS_GATEWAY_ROUTE,
///    SMS_GATEWAY_SENDER_ID — интерпретируются как один провайдер с именем
///    "devil-traff-default" и приоритетом 0.
pub fn load_provider_configs_from_env() -> Vec<SmsProviderConfig> {
    let mut configs = Vec::new();

    for idx in 1..=99u32 {
        let prefix = format!("SMS_PROVIDER_{}_", idx);

        let name = match std::env::var(format!("{}NAME", prefix)).ok().filter(|s| !s.is_empty()) {
            Some(n) => n,
            None => continue,
        };

        let _provider_type = std::env::var(format!("{}TYPE", prefix))
            .ok()
            .filter(|s| !s.is_empty())
            .and_then(|s| ProviderType::from_str(&s))
            .unwrap_or(ProviderType::DevilTraff);

        let base_url = std::env::var(format!("{}URL", prefix))
            .ok()
            .map(|s| s.trim_end_matches('/').to_string())
            .unwrap_or_default();

        let priority = std::env::var(format!("{}PRIORITY", prefix))
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(idx as i32 * 10);

        match _provider_type {
            ProviderType::SkyTelecom => {
                let api_key = std::env::var(format!("{}API_KEY", prefix))
                    .ok()
                    .unwrap_or_default();
                configs.push(SmsProviderConfig::new_sky_telecom(
                    name, priority, base_url, api_key,
                ));
            }
            ProviderType::SmsMobileCc => {
                let username = std::env::var(format!("{}USERNAME", prefix))
                    .ok()
                    .unwrap_or_default();
                let password = std::env::var(format!("{}PASSWORD", prefix))
                    .ok()
                    .unwrap_or_default();
                let route_id = std::env::var(format!("{}ROUTE_ID", prefix))
                    .ok()
                    .filter(|s| !s.is_empty());
                let sender_id = std::env::var(format!("{}SENDER_ID", prefix))
                    .ok()
                    .unwrap_or_else(|| "ElohimSMS".to_string());
                configs.push(SmsProviderConfig::new_smsmobile_cc(
                    name, priority, base_url, username, password, route_id, sender_id,
                ));
            }
            ProviderType::DevilTraff => {
                let auth_token = std::env::var(format!("{}AUTH_TOKEN", prefix))
                    .ok()
                    .unwrap_or_default();
                let route = std::env::var(format!("{}ROUTE", prefix))
                    .ok()
                    .unwrap_or_else(|| "Auto".to_string());
                let sender_id = std::env::var(format!("{}SENDER_ID", prefix))
                    .ok()
                    .unwrap_or_else(|| "ElohimSMS".to_string());
                configs.push(SmsProviderConfig::new_devil_traff(
                    name, priority, base_url, auth_token, route, sender_id,
                ));
            }
        }
    }

    // Fallback на старые переменные.
    if configs.is_empty() {
        if let Ok(base_url) = std::env::var("SMS_GATEWAY_URL") {
            let base_url = base_url.trim_end_matches('/').to_string();
            let auth_token = std::env::var("SMS_GATEWAY_AUTH_TOKEN").unwrap_or_default();
            let route = std::env::var("SMS_GATEWAY_ROUTE").unwrap_or_else(|_| "Auto".to_string());
            let sender_id =
                std::env::var("SMS_GATEWAY_SENDER_ID").unwrap_or_else(|_| "ElohimSMS".to_string());

            configs.push(SmsProviderConfig::new_devil_traff(
                "devil-traff-default",
                0,
                base_url,
                auth_token,
                route,
                sender_id,
            ));
        }
    }

    configs.sort_by_key(|c| c.priority);
    configs
}
