use crate::{
    error::AppError,
    links::{build_short_link, create_link},
    models::{SendCampaignRequest, SendCampaignResponse, SmsLog},
    phone::detect_country_code,
    state::AppState,
    templates::{get_template, render_template},
};
use axum::{
    extract::{Extension, State},
    Json,
};
use tracing::{info, instrument};

#[utoipa::path(
    post,
    path = "/api/v1/campaigns/send",
    request_body = SendCampaignRequest,
    responses(
        (status = 200, description = "Campaign sent", body = SendCampaignResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Too many requests")
    ),
    security(
        ("api_key" = [])
    )
)]
#[instrument(skip(state, payload), fields(phone = %payload.phone))]
pub async fn send_campaign(
    State(state): State<AppState>,
    Extension(api_key_id): Extension<uuid::Uuid>,
    Json(payload): Json<SendCampaignRequest>,
) -> Result<Json<SendCampaignResponse>, AppError> {
    let phone = payload.phone.trim();
    let target_url = payload.url.trim();

    if target_url.is_empty() {
        return Err(AppError::BadRequest("URL is required".to_string()));
    }

    // Определяем страну по номеру
    let country_code = detect_country_code(phone)?;
    info!(country_code = %country_code, "Country detected");

    // Ищем шаблон для страны
    let template = get_template(&state.pool, &country_code)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "No SMS template found for country {}. Please create one via Telegram bot or API.",
                country_code
            ))
        })?;

    // Создаём короткую ссылку
    let campaign = create_link(&state.pool, target_url, phone, &country_code, "").await?;

    let short_link = build_short_link(&state.captcha_site_url, &campaign.short_code);

    // Рендерим сообщение
    let message = render_template(&template.text, &short_link, phone, &country_code);

    // Отправляем SMS
    info!(campaign_id = %campaign.id, "Sending campaign SMS");
    let result = state
        .sms_client
        .send_sms(phone, &message)
        .await
        .map_err(|e| AppError::Internal(format!("SMS gateway error: {e}")))?;

    let status = if result.success { "sent" } else { "failed" };

    // Обновляем campaign
    sqlx::query(
        "UPDATE campaigns SET message = $1, status = $2, provider_response = $3, sent_at = NOW() WHERE id = $4"
    )
    .bind(&message)
    .bind(status)
    .bind(&result.provider_response)
    .bind(campaign.id)
    .execute(&state.pool)
    .await?;

    // Логируем в sms_logs
    sqlx::query_as::<_, SmsLog>(
        "INSERT INTO sms_logs (api_key_id, campaign_id, telegram_id, phone, message, status, provider_response) VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING *"
    )
    .bind(api_key_id)
    .bind(campaign.id)
    .bind(payload.telegram_id)
    .bind(phone)
    .bind(&message)
    .bind(status)
    .bind(&result.provider_response)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(SendCampaignResponse {
        success: result.success,
        campaign_id: campaign.id,
        short_link,
        message,
        provider_response: Some(result.provider_response),
    }))
}

#[utoipa::path(
    post,
    path = "/bot/v1/campaigns/send",
    request_body = SendCampaignRequest,
    responses(
        (status = 200, description = "Campaign sent by admin", body = SendCampaignResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("internal_bot_token" = [])
    )
)]
#[instrument(skip(state, payload), fields(phone = %payload.phone))]
pub async fn send_campaign_as_bot(
    State(state): State<AppState>,
    Json(payload): Json<SendCampaignRequest>,
) -> Result<Json<SendCampaignResponse>, AppError> {
    let phone = payload.phone.trim();
    let target_url = payload.url.trim();

    if target_url.is_empty() {
        return Err(AppError::BadRequest("URL is required".to_string()));
    }

    let country_code = detect_country_code(phone)?;
    let template = get_template(&state.pool, &country_code)
        .await?
        .ok_or_else(|| {
            AppError::BadRequest(format!(
                "No SMS template found for country {}. Please create one via Telegram bot or API.",
                country_code
            ))
        })?;

    let campaign = create_link(&state.pool, target_url, phone, &country_code, "").await?;

    let short_link = build_short_link(&state.captcha_site_url, &campaign.short_code);
    let message = render_template(&template.text, &short_link, phone, &country_code);

    info!(campaign_id = %campaign.id, "Sending campaign SMS via bot");
    let result = state
        .sms_client
        .send_sms(phone, &message)
        .await
        .map_err(|e| AppError::Internal(format!("SMS gateway error: {e}")))?;

    let status = if result.success { "sent" } else { "failed" };

    sqlx::query(
        "UPDATE campaigns SET message = $1, status = $2, provider_response = $3, sent_at = NOW() WHERE id = $4"
    )
    .bind(&message)
    .bind(status)
    .bind(&result.provider_response)
    .bind(campaign.id)
    .execute(&state.pool)
    .await?;

    sqlx::query_as::<_, SmsLog>(
        "INSERT INTO sms_logs (campaign_id, telegram_id, phone, message, status, provider_response) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
    )
    .bind(campaign.id)
    .bind(payload.telegram_id)
    .bind(phone)
    .bind(&message)
    .bind(status)
    .bind(&result.provider_response)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(SendCampaignResponse {
        success: result.success,
        campaign_id: campaign.id,
        short_link,
        message,
        provider_response: Some(result.provider_response),
    }))
}
