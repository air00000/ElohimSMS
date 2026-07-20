use crate::{
    error::AppError,
    models::{ConfigureWebhookRequest, ConfigureWebhookResponse},
    routes::auth::hash_api_key,
    state::AppState,
};

use axum::{
    extract::State,
    http::HeaderMap,
    Json,
};
use chrono::{DateTime, Utc};
use hmac::{Hmac, Mac};
use serde::Serialize;
use sha2::Sha256;
use uuid::Uuid;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize)]
struct LinkVerifiedWebhookEvent {
    event_id: Uuid,
    event_type: &'static str,
    occurred_at: DateTime<Utc>,
    campaign_id: Uuid,
    external_id: Option<String>,
    short_code: String,
    click_count: i32,
}

/// Настроить webhook
///
/// Привязывает webhook к вашему API-ключу. URL обязан быть публичным HTTPS-адресом.
///
/// При переходе получателя по ссылке из SMS на указанный URL отправляется POST-запрос
/// с событием `campaign.link_verified`. Тело запроса подписывается HMAC-SHA256
/// вашим секретом, подпись передаётся в заголовке `X-Elohim-Signature`
/// в формате `sha256=<hex>`. Тип события — в заголовке `X-Elohim-Event`.
///
/// Поля события: `event_id`, `event_type`, `occurred_at`, `campaign_id`,
/// `external_id`, `short_code`, `click_count`.
#[utoipa::path(
    put,
    path = "/api/v1/webhook",
    tag = "webhook",
    request_body = ConfigureWebhookRequest,
    responses(
        (
            status = 200,
            description = "Webhook успешно настроен",
            body = ConfigureWebhookResponse
        ),
        (status = 400, description = "Некорректный URL или секрет короче 16 символов"),
        (status = 401, description = "Отсутствует или невалиден API-ключ")
    ),
    security(("api_key" = []))
)]
pub async fn configure_webhook(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<ConfigureWebhookRequest>,
) -> Result<Json<ConfigureWebhookResponse>, AppError> {
    let raw_key = headers
        .get("X-API-Key")
        .and_then(|value| value.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    // Master key не связан с конкретной записью api_keys,
    // поэтому для него невозможно сохранить отдельный webhook.
    if state.api_key.as_deref() == Some(raw_key) {
        return Err(AppError::BadRequest(
            "Webhook must be configured using a database API key".to_string(),
        ));
    }

    validate_webhook_url(&payload.url)?;

    if payload.secret.len() < 16 {
        return Err(AppError::BadRequest(
            "Webhook secret must contain at least 16 characters".to_string(),
        ));
    }

    let key_hash = hash_api_key(raw_key);
    let api_key_id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id
         FROM api_keys
         WHERE key_hash = $1
           AND is_active = TRUE",
    )
    .bind(&key_hash)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    let is_active = payload.is_active.unwrap_or(true);

    sqlx::query(
        "UPDATE api_keys
         SET webhook_url = $1,
             webhook_secret = $2,
             webhook_is_active = $3
         WHERE id = $4",
    )
    .bind(&payload.url)
    .bind(&payload.secret)
    .bind(is_active)
    .bind(api_key_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(ConfigureWebhookResponse {
        success: true,
        url: payload.url,
        is_active,
    }))
}

fn validate_webhook_url(url: &str) -> Result<(), AppError> {
    let parsed = reqwest::Url::parse(url)
        .map_err(|_| AppError::BadRequest("Invalid webhook URL".to_string()))?;

    if parsed.scheme() != "https" {
        return Err(AppError::BadRequest(
            "Webhook URL must use HTTPS".to_string(),
        ));
    }

    let host = parsed
        .host_str()
        .ok_or_else(|| AppError::BadRequest("Webhook URL has no host".to_string()))?;

    if host.eq_ignore_ascii_case("localhost") {
        return Err(AppError::BadRequest(
            "Local webhook addresses are forbidden".to_string(),
        ));
    }

    // Блокируем очевидные приватные IP-адреса.
    // Для production дополнительно рекомендуется проверять IP после DNS resolve.
    if let Ok(ip) = host.parse::<std::net::IpAddr>() {
        match ip {
            std::net::IpAddr::V4(ip) if ip.is_private() || ip.is_loopback() => {
                return Err(AppError::BadRequest(
                    "Private webhook addresses are forbidden".to_string(),
                ));
            }
            std::net::IpAddr::V6(ip) if ip.is_loopback() => {
                return Err(AppError::BadRequest(
                    "Private webhook addresses are forbidden".to_string(),
                ));
            }
            _ => {}
        }
    }

    Ok(())
}

/// Отправляет событие внешнему сервису.
///
/// Если для API-ключа webhook не настроен или выключен,
/// функция завершается успешно без HTTP-запроса.
pub async fn dispatch_link_verified(
    state: &AppState,
    api_key_id: Uuid,
    campaign_id: Uuid,
    external_id: Option<String>,
    short_code: String,
    click_count: i32,
    occurred_at: DateTime<Utc>,
) -> Result<(), AppError> {
    let webhook = sqlx::query_as::<_, (String, String)>(
        "SELECT webhook_url, webhook_secret
         FROM api_keys
         WHERE id = $1
           AND is_active = TRUE
           AND webhook_is_active = TRUE
           AND webhook_url IS NOT NULL
           AND webhook_secret IS NOT NULL",
    )
    .bind(api_key_id)
    .fetch_optional(&state.pool)
    .await?;

    let Some((webhook_url, webhook_secret)) = webhook else {
        return Ok(());
    };

    let event = LinkVerifiedWebhookEvent {
        event_id: Uuid::new_v4(),
        event_type: "campaign.link_verified",
        occurred_at,
        campaign_id,
        external_id,
        short_code,
        click_count,
    };

    // Важно: подписываются именно байты HTTP-body.
    let body = serde_json::to_vec(&event)
        .map_err(|error| AppError::Internal(format!(
            "Failed to serialize webhook event: {error}"
        )))?;

    let mut mac = HmacSha256::new_from_slice(webhook_secret.as_bytes())
        .map_err(|error| AppError::Internal(format!(
            "Failed to initialize webhook signature: {error}"
        )))?;

    mac.update(&body);

    let signature = format!(
        "sha256={}",
        hex::encode(mac.finalize().into_bytes())
    );

    let response = state
        .http_client
        .post(&webhook_url)
        .header("Content-Type", "application/json")
        .header("X-Elohim-Event", "campaign.link_verified")
        .header("X-Elohim-Signature", signature)
        .body(body)
        .send()
        .await
        .map_err(|error| AppError::Internal(format!(
            "Webhook request failed: {error}"
        )))?;

    if !response.status().is_success() {
        return Err(AppError::Internal(format!(
            "Webhook returned HTTP {}",
            response.status()
        )));
    }

    tracing::info!(
        api_key_id = %api_key_id,
        campaign_id = %campaign_id,
        webhook_url = %webhook_url,
        "Webhook delivered"
    );

    Ok(())
}
