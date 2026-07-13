use crate::{
    error::AppError,
    models::{Campaign, SendSmsRequest, SendSmsResponse, SmsLog, Template},
    phone::{detect_country_code, normalize_phone},
    routes::{auth::hash_api_key, bot::render_template},
    state::AppState,
};
use axum::{extract::State, http::HeaderMap, Json};
// HeaderMap используется из axum::http выше
use rand::Rng as _;
use serde_json::Value;
use tracing::{info, instrument};
use uuid::Uuid;

struct SendOutcome {
    success: bool,
    message: String,
    provider_response: Value,
    provider_name: Option<String>,
    campaign_id: Option<Uuid>,
    short_link: Option<String>,
}

#[utoipa::path(
    post,
    path = "/api/v1/sms/send",
    request_body = SendSmsRequest,
    responses(
        (status = 200, description = "SMS sent or queued", body = SendSmsResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Too many requests")
    ),
    security(
        ("api_key" = [])
    )
)]
#[instrument(skip(state, headers, payload), fields(phone = %payload.phone))]
pub async fn send_sms(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<SendSmsRequest>,
) -> Result<Json<SendSmsResponse>, AppError> {
    let phone = normalize_phone(&payload.phone)?;
    let target_url = payload.message.trim();

    if target_url.is_empty() {
        return Err(AppError::BadRequest("Message is required".to_string()));
    }

    let (api_key_id, _sender_name) = resolve_key_owner(&state, &headers).await?;
    let sender_id = payload
        .sender_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("TRACKING");
    let country_code = detect_country_code(&phone)?;

    let template = sqlx::query_as::<_, Template>(
        "SELECT * FROM templates WHERE country_code = $1 AND is_active = TRUE AND is_favorite = TRUE LIMIT 1",
    )
    .bind(&country_code)
    .fetch_optional(&state.pool)
    .await?;

    let outcome = if let Some(template) = template {
        send_api_campaign(
            &state,
            &phone,
            target_url,
            &country_code,
            &template,
            api_key_id,
            payload.external_id.as_deref(),
            sender_id,
        )
        .await?
    } else {
        send_api_plain_sms(
            &state,
            &phone,
            target_url,
            api_key_id,
            sender_id,
        )
        .await?
    };

    let status = if outcome.success { "sent" } else { "failed" };
    sqlx::query_as::<_, SmsLog>(
        "INSERT INTO sms_logs (phone, message, status, provider_response, provider_name, api_key_id, campaign_id)
         VALUES ($1, $2, $3, $4, $5, $6, $7)
         RETURNING id, phone, message, status, provider_response, provider_name, created_at"
    )
    .bind(&phone)
    .bind(&outcome.message)
    .bind(status)
    .bind(&outcome.provider_response)
    .bind(&outcome.provider_name)
    .bind(api_key_id)
    .bind(outcome.campaign_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(SendSmsResponse {
        success: outcome.success,
        message: if outcome.success {
            if outcome.campaign_id.is_some() {
                "Campaign sent via API".to_string()
            } else {
                "SMS accepted by gateway".to_string()
            }
        } else {
            "SMS gateway rejected the message".to_string()
        },
        provider_response: Some(outcome.provider_response),
        campaign_id: outcome.campaign_id,
        short_link: outcome.short_link,
    }))
}

async fn send_api_campaign(
    state: &AppState,
    phone: &str,
    target_url: &str,
    country_code: &str,
    template: &Template,
    api_key_id: Option<Uuid>,
    external_id: Option<&str>,
    sender_id: &str,
) -> Result<SendOutcome, AppError> {
    let short_code = generate_short_code(state).await?;
    let short_link = state.config.short_link(&short_code);

    let rendered = render_template(&template.text, &short_link, phone, country_code);
    let template_name = template.name.clone();

    info!(phone = %phone, short_code = %short_code, "Sending API campaign");

    let result = state
        .sms_client
        .send_sms(phone, &rendered, Some(sender_id))
        .await
        .map_err(|e| AppError::Internal(format!("SMS gateway error: {e}")))?;

    let status = if result.success { "sent" } else { "failed" };
    let sent_at = if result.success { Some(chrono::Utc::now()) } else { None };
    let provider_name = Some(result.provider_name.clone());
    let provider_response = result.provider_response_json();

    let campaign = sqlx::query_as::<_, Campaign>(
        "INSERT INTO campaigns
         (short_code, target_url, phone, country_code, message, template_name, status, api_key_id, external_id, provider_response, provider_name, sent_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
         RETURNING *"
    )
    .bind(&short_code)
    .bind(target_url)
    .bind(phone)
    .bind(country_code)
    .bind(&rendered)
    .bind(template_name.as_deref())
    .bind(status)
    .bind(api_key_id)
    .bind(external_id)
    .bind(&provider_response)
    .bind(&provider_name)
    .bind(sent_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(SendOutcome {
        success: result.success,
        message: rendered,
        provider_response,
        provider_name,
        campaign_id: Some(campaign.id),
        short_link: Some(short_link),
    })
}

async fn send_api_plain_sms(
    state: &AppState,
    phone: &str,
    message: &str,
    _api_key_id: Option<Uuid>,
    sender_id: &str,
) -> Result<SendOutcome, AppError> {
    info!(phone = %phone, "Sending plain SMS via API");

    let result = state
        .sms_client
        .send_sms(phone, message, Some(sender_id))
        .await
        .map_err(|e| AppError::Internal(format!("SMS gateway error: {e}")))?;

    let provider_name = Some(result.provider_name.clone());
    let provider_response = result.provider_response_json();

    Ok(SendOutcome {
        success: result.success,
        message: message.to_string(),
        provider_response,
        provider_name,
        campaign_id: None,
        short_link: None,
    })
}

async fn resolve_key_owner(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<(Option<Uuid>, Option<String>), AppError> {
    let key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if state.api_key.as_deref() == Some(key) {
        return Ok((None, None));
    }

    let key_hash = hash_api_key(key);
    let row = sqlx::query_as::<_, (Uuid, Option<i64>)>(
        "SELECT id, created_by_telegram_id FROM api_keys WHERE key_hash = $1 AND is_active = TRUE"
    )
    .bind(&key_hash)
    .fetch_optional(&state.pool)
    .await?;

    let (id, owner_tid) = row.ok_or(AppError::Unauthorized)?;

    let sender_name = if let Some(tid) = owner_tid {
        sqlx::query_scalar::<_, Option<String>>(
            "SELECT sender_name FROM admins WHERE telegram_id = $1"
        )
        .bind(tid)
        .fetch_optional(&state.pool)
        .await?
        .flatten()
    } else {
        None
    };

    Ok((Some(id), sender_name))
}

async fn generate_short_code(state: &AppState) -> Result<String, AppError> {
    loop {
        let code: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();

        let exists = sqlx::query_scalar::<_, bool>(
            "SELECT EXISTS(SELECT 1 FROM campaigns WHERE short_code = $1)",
        )
        .bind(&code)
        .fetch_one(&state.pool)
        .await?;

        if !exists {
            return Ok(code);
        }
    }
}
