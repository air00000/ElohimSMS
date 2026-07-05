use crate::config::Config;

#[derive(Debug, Clone)]
pub struct SmsGatewayConfig {
    pub base_url: String,
    pub auth_token: String,
    pub route: String,
    pub sender_id: String,
}

impl From<&Config> for SmsGatewayConfig {
    fn from(config: &Config) -> Self {
        Self {
            base_url: config.sms_gateway_url.clone(),
            auth_token: config.sms_gateway_auth_token.clone(),
            route: config.sms_gateway_route.clone(),
            sender_id: config.sms_gateway_sender_id.clone(),
        }
    }
}
