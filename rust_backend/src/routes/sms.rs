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

/// Отправить SMS со ссылкой
///
/// Отправляет SMS на указанный номер. Текст сообщения — избранный шаблон
/// для страны получателя (страна определяется по номеру): ссылка из поля
/// `message` оборачивается в короткую и подставляется в текст шаблона.
/// Имя отправителя — избранное имя отправителя для этой страны.
///
/// Если для страны не настроен избранный шаблон или избранное имя
/// отправителя, SMS не отправляется — возвращается ошибка 400.
///
/// При переходе получателя по ссылке на ваш webhook отправляется событие
/// `campaign.link_verified` (см. `PUT /api/v1/webhook`).
#[utoipa::path(
    post,
    path = "/api/v1/sms/send",
    tag = "sms",
    request_body = SendSmsRequest,
    responses(
        (status = 200, description = "SMS принята в обработку", body = SendSmsResponse),
        (status = 400, description = "Некорректный запрос (невалидный номер, пустое сообщение) или для страны не настроен шаблон/имя отправителя"),
        (status = 401, description = "Отсутствует или невалиден API-ключ"),
        (status = 429, description = "Превышен лимит запросов")
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

    let api_key_id = resolve_key_owner(&state, &headers).await?;
    let country_code = detect_country_code(&phone)?;

    let template = sqlx::query_as::<_, Template>(
        "SELECT * FROM templates WHERE country_code = $1 AND is_active = TRUE AND is_favorite = TRUE LIMIT 1",
    )
    .bind(&country_code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        AppError::BadRequest(format!(
            "No favorite template configured for country {country_code}"
        ))
    })?;

    let sender_name = sqlx::query_scalar::<_, String>(
        "SELECT name FROM sender_names WHERE country_code = $1 AND is_active = TRUE AND is_favorite = TRUE LIMIT 1",
    )
    .bind(&country_code)
    .fetch_optional(&state.pool)
    .await?
    .ok_or_else(|| {
        AppError::BadRequest(format!(
            "No sender name configured for country {country_code}"
        ))
    })?;

    let outcome = send_api_campaign(
        &state,
        &phone,
        target_url,
        &country_code,
        &template,
        api_key_id,
        payload.external_id.as_deref(),
        &sender_name,
    )
    .await?;

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

async fn resolve_key_owner(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<Option<Uuid>, AppError> {
    let key = headers
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if state.api_key.as_deref() == Some(key) {
        return Ok(None);
    }

    let key_hash = hash_api_key(key);
    let id = sqlx::query_scalar::<_, Uuid>(
        "SELECT id FROM api_keys WHERE key_hash = $1 AND is_active = TRUE",
    )
    .bind(&key_hash)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::Unauthorized)?;

    Ok(Some(id))
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
