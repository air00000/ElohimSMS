use crate::{
    error::AppError,
    models::{SendSmsRequest, SendSmsResponse, SmsLog},
    phone::normalize_phone,
    state::AppState,
};
use axum::{
    extract::{Extension, State},
    Json,
};
use serde_json::Value;
use tracing::{info, instrument};

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
#[instrument(skip(state, payload), fields(phone = %payload.phone))]
pub async fn send_sms(
    State(state): State<AppState>,
    Extension(api_key_id): Extension<uuid::Uuid>,
    Json(payload): Json<SendSmsRequest>,
) -> Result<Json<SendSmsResponse>, AppError> {
    let phone = normalize_phone(&payload.phone)?;
    let message = payload.message.trim();

    if message.is_empty() {
        return Err(AppError::BadRequest("Message is required".to_string()));
    }

    info!(api_key_id = %api_key_id, "Sending SMS");

    let result = state
        .sms_client
        .send_sms(&phone, message)
        .await
        .map_err(|e| AppError::Internal(format!("SMS gateway error: {e}")))?;

    let status = if result.success { "sent" } else { "failed" };

    sqlx::query_as::<_, SmsLog>(
        "INSERT INTO sms_logs (api_key_id, telegram_id, phone, message, status, provider_response) VALUES ($1, $2, $3, $4, $5, $6) RETURNING *"
    )
    .bind(api_key_id)
    .bind(payload.telegram_id)
    .bind(phone)
    .bind(message)
    .bind(status)
    .bind(&result.provider_response)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(SendSmsResponse {
        success: result.success,
        message: if result.success {
            "SMS accepted by gateway".to_string()
        } else {
            "SMS gateway rejected the message".to_string()
        },
        provider_response: Some(result.provider_response),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/sms/balance",
    responses(
        (status = 200, description = "Gateway balance", body = Value),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Too many requests")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn get_balance(
    State(state): State<AppState>,
    Extension(_api_key_id): Extension<uuid::Uuid>,
) -> Result<Json<Value>, AppError> {
    let balance = state
        .sms_client
        .get_balance()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get balance: {e}")))?;

    Ok(Json(balance))
}

#[utoipa::path(
    get,
    path = "/api/v1/sms/routes",
    responses(
        (status = 200, description = "Available routes", body = Value),
        (status = 401, description = "Unauthorized"),
        (status = 429, description = "Too many requests")
    ),
    security(
        ("api_key" = [])
    )
)]
pub async fn get_routes(
    State(state): State<AppState>,
    Extension(_api_key_id): Extension<uuid::Uuid>,
) -> Result<Json<Value>, AppError> {
    let routes = state
        .sms_client
        .get_routes()
        .await
        .map_err(|e| AppError::Internal(format!("Failed to get routes: {e}")))?;

    Ok(Json(routes))
}
