use crate::{
    error::AppError,
    models::{
        Admin, ApiKey, ApiKeyListItem, BotSendSmsRequest, Campaign, CreateAdminRequest,
        CreateKeyRequest, CreateKeyResponse, CreateSenderNameRequest, CreateTemplateRequest,
        EnsureOwnerRequest, SenderName, SendSmsResponse, StatsResponse, Template,
        UpdateSenderNameRequest,
    },
    phone::{detect_country_code, normalize_phone},
    routes::auth::hash_api_key,
    state::AppState,
};
use axum::{
    extract::{Path, State},
    Json,
};
use rand::Rng as _;
use serde_json::json;
use sqlx::FromRow;
use tracing::{info, instrument};
use uuid::Uuid;

/// Рендерит шаблон, подставляя placeholders.
pub(crate) fn render_template(template: &str, link: &str, phone: &str, country: &str) -> String {
    template
        .replace("{link}", link)
        .replace("{phone}", phone)
        .replace("{country}", country)
}

/// Генерирует уникальный короткий код.
async fn generate_short_code(pool: &sqlx::PgPool) -> Result<String, AppError> {
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
        .fetch_one(pool)
        .await?;

        if !exists {
            return Ok(code);
        }
    }
}

/// Отправляет уведомление админу через внутренний endpoint бота.
pub(crate) fn notify_admin(state: &AppState, telegram_id: i64, text: String) {
    let client = state.http_client.clone();
    let url = format!("{}/internal/notify", state.bot_internal_url);
    let token = state.internal_bot_token.clone();

    tracing::info!(
        telegram_id,
        url = %url,
        "notify_admin called, spawning notification task"
    );

    tokio::spawn(async move {
        let payload = json!({
            "telegram_id": telegram_id,
            "text": text,
        });

        match client
            .post(&url)
            .header("X-Internal-Bot-Token", token)
            .json(&payload)
            .send()
            .await
        {
            Ok(response) => {
                let status = response.status();
                if status.is_success() || status == 204 {
                    tracing::info!(telegram_id, "Admin notified about link click");
                } else {
                    let body = response.text().await.unwrap_or_default();
                    tracing::warn!(
                        telegram_id,
                        status = %status,
                        body = %body,
                        "Failed to notify admin: bot returned error"
                    );
                }
            }
            Err(e) => {
                tracing::warn!(
                    telegram_id,
                    error = %e,
                    url = %url,
                    "Failed to notify admin: request error"
                );
            }
        }
    });
}

// ---------- Администраторы ----------

/// Уведомляет владельца сервиса о первом переходе по кампании с указанием клиента.
///
/// `already_notified` — telegram_id, которому уже отправлено обычное уведомление;
/// если владелец совпадает с ним, повторное сообщение не шлём.
/// Возвращает telegram_id владельца, если он задан.
pub(crate) async fn notify_owner_first_click(
    state: &AppState,
    campaign_id: Uuid,
    phone: &str,
    country_code: &str,
    message: &str,
    template_name: Option<&str>,
    sent_by_telegram_id: Option<i64>,
    api_key_id: Option<Uuid>,
    already_notified: Option<i64>,
) -> Option<i64> {
    let owner_id = match get_owner_telegram_id(&state.pool).await {
        Ok(id) => id,
        Err(e) => {
            tracing::warn!(error = %e, "Failed to resolve service owner for click notification");
            None
        }
    }?;

    if already_notified == Some(owner_id) {
        return Some(owner_id);
    }

    let client_line = if let Some(tid) = sent_by_telegram_id {
        let username = sqlx::query_scalar::<_, Option<String>>(
            "SELECT username FROM admins WHERE telegram_id = $1",
        )
        .bind(tid)
        .fetch_optional(&state.pool)
        .await
        .ok()
        .flatten()
        .flatten();
        match username {
            Some(u) if !u.is_empty() => format!("Telegram: @{} (id {})", u, tid),
            _ => format!("Telegram: id {}", tid),
        }
    } else if let Some(key_id) = api_key_id {
        let name = sqlx::query_scalar::<_, String>("SELECT name FROM api_keys WHERE id = $1")
            .bind(key_id)
            .fetch_optional(&state.pool)
            .await
            .ok()
            .flatten();
        match name {
            Some(n) => format!("API-ключ: «{}»", n),
            None => "API-ключ: не найден".to_string(),
        }
    } else {
        "неизвестен".to_string()
    };

    let template_line = if let Some(name) = template_name {
        format!("\n<b>Шаблон:</b> {}", name)
    } else {
        String::new()
    };

    let text = format!(
        "👑 <b>Переход у клиента</b>\n\n<b>Клиент:</b> {}\n<b>Кампания:</b> <code>{}</code>\n<b>Номер:</b> <code>{}</code>\n<b>Страна:</b> {}\n<b>Сообщение:</b> <code>{}</code>{}\n<b>Время:</b> {}",
        client_line,
        campaign_id,
        phone,
        country_code,
        message,
        template_line,
        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
    );

    tracing::info!(
        telegram_id = owner_id,
        campaign_id = %campaign_id,
        "Notifying service owner about client click"
    );
    notify_admin(state, owner_id, text);

    Some(owner_id)
}

#[utoipa::path(
    get,
    path = "/bot/v1/admin",
    responses((status = 200, description = "List admins", body = Vec<Admin>))
)]
pub async fn list_admins(State(state): State<AppState>) -> Result<Json<Vec<Admin>>, AppError> {
    let admins = sqlx::query_as::<_, Admin>("SELECT * FROM admins ORDER BY created_at")
        .fetch_all(&state.pool)
        .await?;
    Ok(Json(admins))
}

#[utoipa::path(
    post,
    path = "/bot/v1/admin",
    request_body = CreateAdminRequest,
    responses((status = 200, description = "Admin created", body = Admin))
)]
pub async fn create_admin(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdminRequest>,
) -> Result<Json<Admin>, AppError> {
    let username = payload.username.as_deref().map(|u| u.trim());
    let username = if username.map(|u| u.is_empty()).unwrap_or(true) {
        None
    } else {
        Some(username.unwrap().to_string())
    };

    let admin = sqlx::query_as::<_, Admin>(
        "INSERT INTO admins (telegram_id, username)
         VALUES ($1, $2)
         ON CONFLICT (telegram_id) DO UPDATE SET username = EXCLUDED.username
         RETURNING *",
    )
    .bind(payload.telegram_id)
    .bind(username)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(admin))
}

#[utoipa::path(
    post,
    path = "/bot/v1/admin/me/sender_name",
    request_body = UpdateSenderNameRequest,
    responses((status = 200, description = "Sender name updated", body = Admin))
)]
pub async fn update_sender_name(
    State(state): State<AppState>,
    Json(payload): Json<UpdateSenderNameRequest>,
) -> Result<Json<Admin>, AppError> {
    let sender_name = payload.sender_name.as_deref().map(|s| s.trim());
    let sender_name = if sender_name.map(|s| s.is_empty()).unwrap_or(true) {
        None
    } else {
        Some(sender_name.unwrap().to_string())
    };

    let admin = sqlx::query_as::<_, Admin>(
        "UPDATE admins SET sender_name = $1 WHERE telegram_id = $2 RETURNING *"
    )
    .bind(sender_name)
    .bind(payload.telegram_id)
    .fetch_optional(&state.pool)
    .await?
    .ok_or(AppError::NotFound)?;

    Ok(Json(admin))
}

#[utoipa::path(
    post,
    path = "/bot/v1/admin/ensure_owner",
    request_body = EnsureOwnerRequest,
    responses((status = 200, description = "Owner ensured", body = Admin))
)]
pub async fn ensure_owner(
    State(state): State<AppState>,
    Json(payload): Json<EnsureOwnerRequest>,
) -> Result<Json<Admin>, AppError> {
    let username = payload.username.as_deref().map(|u| u.trim());
    let username = if username.map(|u| u.is_empty()).unwrap_or(true) {
        None
    } else {
        Some(username.unwrap().to_string())
    };

    let admin = sqlx::query_as::<_, Admin>(
        "INSERT INTO admins (telegram_id, username, is_owner)
         VALUES ($1, $2, TRUE)
         ON CONFLICT (telegram_id) DO UPDATE SET is_owner = TRUE, username = EXCLUDED.username
         RETURNING *"
    )
    .bind(payload.telegram_id)
    .bind(username)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(admin))
}

#[utoipa::path(
    delete,
    path = "/bot/v1/admin/{telegram_id}",
    responses((status = 204, description = "Admin deleted"))
)]
pub async fn remove_admin(
    State(state): State<AppState>,
    Path(telegram_id): Path<i64>,
) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM admins WHERE telegram_id = $1 AND is_owner = FALSE")
        .bind(telegram_id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}

// ---------- API-ключи ----------

#[utoipa::path(
    get,
    path = "/bot/v1/keys",
    responses((status = 200, description = "List API keys", body = Vec<ApiKeyListItem>))
)]
pub async fn list_keys(
    State(state): State<AppState>,
) -> Result<Json<Vec<ApiKeyListItem>>, AppError> {
    let keys = sqlx::query_as::<_, ApiKeyListItem>(
        "SELECT id, name, is_active, created_by_telegram_id, created_at, last_used_at
         FROM api_keys ORDER BY created_at DESC"
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(keys))
}

#[utoipa::path(
    post,
    path = "/bot/v1/keys",
    request_body = CreateKeyRequest,
    responses((status = 200, description = "Key created", body = CreateKeyResponse))
)]
pub async fn create_key(
    State(state): State<AppState>,
    Json(payload): Json<CreateKeyRequest>,
) -> Result<Json<CreateKeyResponse>, AppError> {
    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Key name is required".to_string()));
    }

    let key = Uuid::new_v4().to_string();
    let key_hash = hash_api_key(&key);

    let record = sqlx::query_as::<_, ApiKey>(
        "INSERT INTO api_keys (key_hash, name, created_by_telegram_id)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(&key_hash)
    .bind(name)
    .bind(payload.created_by_telegram_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CreateKeyResponse {
        id: record.id,
        name: record.name,
        key,
    }))
}

#[utoipa::path(
    post,
    path = "/bot/v1/keys/{id}/revoke",
    responses((status = 204, description = "Key revoked"))
)]
pub async fn revoke_key(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let result = sqlx::query("UPDATE api_keys SET is_active = FALSE WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}

// ---------- Шаблоны ----------

#[utoipa::path(
    get,
    path = "/bot/v1/templates",
    responses((status = 200, description = "List templates", body = Vec<Template>))
)]
pub async fn list_templates(
    State(state): State<AppState>,
) -> Result<Json<Vec<Template>>, AppError> {
    let templates =
        sqlx::query_as::<_, Template>("SELECT * FROM templates ORDER BY country_code, created_at")
            .fetch_all(&state.pool)
            .await?;
    Ok(Json(templates))
}

#[utoipa::path(
    post,
    path = "/bot/v1/templates",
    request_body = CreateTemplateRequest,
    responses((status = 200, description = "Template created", body = Template))
)]
pub async fn create_template(
    State(state): State<AppState>,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<Json<Template>, AppError> {
    let country_code = payload.country_code.trim().to_uppercase();
    if country_code.len() != 2 {
        return Err(AppError::BadRequest(
            "Country code must be 2 letters".to_string(),
        ));
    }

    let text = payload.text.trim();
    if text.is_empty() {
        return Err(AppError::BadRequest("Template text is required".to_string()));
    }

    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Template name is required".to_string()));
    }

    let has_favorite = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM templates WHERE country_code = $1 AND is_favorite = TRUE)"
    )
    .bind(&country_code)
    .fetch_one(&state.pool)
    .await?;

    let template = sqlx::query_as::<_, Template>(
        "INSERT INTO templates (country_code, name, text, is_favorite)
         VALUES ($1, $2, $3, $4)
         RETURNING *",
    )
    .bind(&country_code)
    .bind(name.to_string())
    .bind(text)
    .bind(!has_favorite)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(template))
}

#[utoipa::path(
    delete,
    path = "/bot/v1/templates/{id}",
    responses((status = 204, description = "Template deleted"))
)]
pub async fn delete_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM templates WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}

#[utoipa::path(
    post,
    path = "/bot/v1/templates/{id}/favorite",
    responses((status = 200, description = "Favorite set", body = Template))
)]
pub async fn set_favorite_template(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Template>, AppError> {
    let template = sqlx::query_as::<_, Template>("SELECT * FROM templates WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut tx = state.pool.begin().await?;

    sqlx::query("UPDATE templates SET is_favorite = FALSE WHERE country_code = $1")
        .bind(&template.country_code)
        .execute(&mut *tx)
        .await?;

    let updated = sqlx::query_as::<_, Template>(
        "UPDATE templates SET is_favorite = TRUE, updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(updated))
}

// ---------- Имена отправителя ----------

#[utoipa::path(
    get,
    path = "/bot/v1/sender-names",
    responses((status = 200, description = "List sender names", body = Vec<SenderName>))
)]
pub async fn list_sender_names(
    State(state): State<AppState>,
) -> Result<Json<Vec<SenderName>>, AppError> {
    let names = sqlx::query_as::<_, SenderName>(
        "SELECT * FROM sender_names ORDER BY country_code, created_at",
    )
    .fetch_all(&state.pool)
    .await?;
    Ok(Json(names))
}

#[utoipa::path(
    post,
    path = "/bot/v1/sender-names",
    request_body = CreateSenderNameRequest,
    responses((status = 200, description = "Sender name created", body = SenderName))
)]
pub async fn create_sender_name(
    State(state): State<AppState>,
    Json(payload): Json<CreateSenderNameRequest>,
) -> Result<Json<SenderName>, AppError> {
    let country_code = payload.country_code.trim().to_uppercase();
    if country_code.len() != 2 {
        return Err(AppError::BadRequest(
            "Country code must be 2 letters".to_string(),
        ));
    }

    let name = payload.name.trim();
    if name.is_empty() {
        return Err(AppError::BadRequest("Sender name is required".to_string()));
    }
    if name.len() > 11 || !name.chars().all(|c| c.is_ascii_alphanumeric()) {
        return Err(AppError::BadRequest(
            "Sender name must be up to 11 latin letters/digits".to_string(),
        ));
    }

    let has_favorite = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM sender_names WHERE country_code = $1 AND is_favorite = TRUE)"
    )
    .bind(&country_code)
    .fetch_one(&state.pool)
    .await?;

    let sender_name = sqlx::query_as::<_, SenderName>(
        "INSERT INTO sender_names (country_code, name, is_favorite)
         VALUES ($1, $2, $3)
         RETURNING *",
    )
    .bind(&country_code)
    .bind(name.to_string())
    .bind(!has_favorite)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(sender_name))
}

#[utoipa::path(
    delete,
    path = "/bot/v1/sender-names/{id}",
    responses((status = 204, description = "Sender name deleted"))
)]
pub async fn delete_sender_name(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM sender_names WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(())
}

#[utoipa::path(
    post,
    path = "/bot/v1/sender-names/{id}/favorite",
    responses((status = 200, description = "Favorite set", body = SenderName))
)]
pub async fn set_favorite_sender_name(
    State(state): State<AppState>,
    Path(id): Path<Uuid>,
) -> Result<Json<SenderName>, AppError> {
    let sender_name = sqlx::query_as::<_, SenderName>("SELECT * FROM sender_names WHERE id = $1")
        .bind(id)
        .fetch_optional(&state.pool)
        .await?
        .ok_or(AppError::NotFound)?;

    let mut tx = state.pool.begin().await?;

    sqlx::query("UPDATE sender_names SET is_favorite = FALSE WHERE country_code = $1")
        .bind(&sender_name.country_code)
        .execute(&mut *tx)
        .await?;

    let updated = sqlx::query_as::<_, SenderName>(
        "UPDATE sender_names SET is_favorite = TRUE, updated_at = NOW() WHERE id = $1 RETURNING *",
    )
    .bind(id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;
    Ok(Json(updated))
}

// ---------- Отправка SMS из бота ----------

#[utoipa::path(
    post,
    path = "/bot/v1/sms/send",
    request_body = BotSendSmsRequest,
    responses((status = 200, description = "SMS sent", body = SendSmsResponse))
)]
#[instrument(skip(state, payload), fields(phone = %payload.phone))]
pub async fn bot_send_sms(
    State(state): State<AppState>,
    Json(payload): Json<BotSendSmsRequest>,
) -> Result<Json<SendSmsResponse>, AppError> {
    let phone = normalize_phone(&payload.phone)?;
    let message = payload.message.trim();
    if message.is_empty() {
        return Err(AppError::BadRequest("Message is required".to_string()));
    }

    let sender_id = payload
        .sender_id
        .as_deref()
        .filter(|s| !s.is_empty())
        .unwrap_or("TRACKING");

    // Если указан URL и сообщение содержит {link} — отправляем как кампанию с короткой ссылкой.
    if let Some(target_url) = payload.url {
        let target_url = target_url.trim();
        if !target_url.is_empty() && message.contains("{link}") {
            return send_bot_link_campaign(&state, phone, message, target_url, payload.telegram_id, payload.template_name, sender_id).await;
        }
    }

    info!(phone = %phone, "Sending SMS from bot");

    let result = state
        .sms_client
        .send_sms(&phone, message, Some(sender_id))
        .await
        .map_err(|e| AppError::Internal(format!("SMS gateway error: {e}")))?;

    let status = if result.success { "sent" } else { "failed" };
    let provider_response = result.provider_response_json();

    sqlx::query(
        "INSERT INTO sms_logs (phone, message, status, provider_response, provider_name, telegram_id)
         VALUES ($1, $2, $3, $4, $5, $6)"
    )
    .bind(&phone)
    .bind(message)
    .bind(status)
    .bind(&provider_response)
    .bind(&result.provider_name)
    .bind(payload.telegram_id)
    .execute(&state.pool)
    .await?;

    Ok(Json(SendSmsResponse {
        success: result.success,
        message: if result.success {
            "SMS accepted by gateway".to_string()
        } else {
            "SMS gateway rejected the message".to_string()
        },
        provider_response: Some(provider_response),
        campaign_id: None,
        short_link: None,
    }))
}

async fn send_bot_link_campaign(
    state: &AppState,
    phone: String,
    message: &str,
    target_url: &str,
    telegram_id: i64,
    template_name: Option<String>,
    sender_id: &str,
) -> Result<Json<SendSmsResponse>, AppError> {
    let country_code = detect_country_code(&phone)?;
    let short_code = generate_short_code(&state.pool).await?;

    let short_link = state.config.short_link(&short_code);

    let rendered = message.replace("{link}", &short_link);
    info!(phone = %phone, short_code = %short_code, "Sending bot link campaign");

    let result = state
        .sms_client
        .send_sms(&phone, &rendered, Some(sender_id))
        .await
        .map_err(|e| AppError::Internal(format!("SMS gateway error: {e}")))?;

    let status = if result.success { "sent" } else { "failed" };
    let sent_at = if result.success { Some(chrono::Utc::now()) } else { None };
    let provider_response = result.provider_response_json();

    let campaign = sqlx::query_as::<_, Campaign>(
        "INSERT INTO campaigns
         (short_code, target_url, phone, country_code, message, template_name, status, sent_by_telegram_id, provider_response, provider_name, sent_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
         RETURNING *"
    )
    .bind(&short_code)
    .bind(target_url)
    .bind(&phone)
    .bind(&country_code)
    .bind(&rendered)
    .bind(template_name.as_deref())
    .bind(status)
    .bind(telegram_id)
    .bind(&provider_response)
    .bind(&result.provider_name)
    .bind(sent_at)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(SendSmsResponse {
        success: result.success,
        message: if result.success {
            "Campaign with short link accepted by gateway".to_string()
        } else {
            "SMS gateway rejected the message".to_string()
        },
        provider_response: Some(provider_response),
        campaign_id: Some(campaign.id),
        short_link: Some(short_link),
    }))
}

// ---------- Редирект и уведомления ----------

#[derive(FromRow)]
struct CampaignNotifyInfo {
    id: Uuid,
    target_url: String,
    phone: String,
    country_code: String,
    message: String,
    template_name: Option<String>,
    sent_by_telegram_id: Option<i64>,
    api_key_id: Option<Uuid>,
}

async fn get_owner_telegram_id(pool: &sqlx::PgPool) -> Result<Option<i64>, AppError> {
    let owner = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT telegram_id FROM admins WHERE is_owner = TRUE LIMIT 1"
    )
    .fetch_optional(pool)
    .await?;
    Ok(owner.flatten())
}

pub async fn redirect(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<axum::response::Redirect, AppError> {
    tracing::info!(short_code = %short_code, "Redirect request received");

    let campaign = sqlx::query_as::<_, CampaignNotifyInfo>(
        "SELECT id, target_url, phone, country_code, message, template_name, sent_by_telegram_id, api_key_id
         FROM campaigns WHERE short_code = $1"
    )
    .bind(&short_code)
    .fetch_optional(&state.pool)
    .await?;

    let campaign = match campaign {
        Some(c) => c,
        None => {
            tracing::warn!(short_code = %short_code, "Campaign not found for redirect");
            return Err(AppError::NotFound);
        }
    };

    let mut admin_telegram_id = if let Some(tid) = campaign.sent_by_telegram_id {
        Some(tid)
    } else if let Some(key_id) = campaign.api_key_id {
        sqlx::query_scalar::<_, Option<i64>>(
            "SELECT created_by_telegram_id FROM api_keys WHERE id = $1"
        )
        .bind(key_id)
        .fetch_optional(&state.pool)
        .await?
        .flatten()
    } else {
        None
    };

    // Если отправителя не нашли — пытаемся уведомить владельца сервиса.
    if admin_telegram_id.is_none() {
        admin_telegram_id = get_owner_telegram_id(&state.pool).await?;
    }

    // click_count = 1 после инкремента означает, что до этого переходов не было.
    let is_first_click = sqlx::query_scalar::<_, bool>(
        r#"
        UPDATE campaigns
        SET
            click_count = click_count + 1,
            first_clicked_at = COALESCE(first_clicked_at, NOW())
        WHERE id = $1
        RETURNING click_count = 1 AS is_first_click
        "#,
    )
    .bind(campaign.id)
    .fetch_one(&state.pool)
    .await?;

    sqlx::query(
        "INSERT INTO campaign_clicks (campaign_id) VALUES ($1)"
    )
    .bind(campaign.id)
    .execute(&state.pool)
    .await?;

    if let Some(tid) = admin_telegram_id {
        tracing::info!(
            admin_telegram_id = tid,
            campaign_id = %campaign.id,
            "Sending click notification to admin"
        );
        let template_line = if let Some(name) = &campaign.template_name {
            format!("\n<b>Шаблон:</b> {}", name)
        } else {
            String::new()
        };

        let text = format!(
            "🔗 <b>Переход по ссылке</b>\n\n<b>Кампания:</b> <code>{}</code>\n<b>Номер:</b> <code>{}</code>\n<b>Страна:</b> {}\n<b>Сообщение:</b> <code>{}</code>{}\n<b>Время:</b> {}",
            campaign.id,
            campaign.phone,
            campaign.country_code,
            campaign.message,
            template_line,
            chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")
        );
        notify_admin(&state, tid, text);
    }

    if is_first_click {
        // Владелец сервиса получает копию с указанием клиента
        // (если он не является получателем обычного уведомления).
        notify_owner_first_click(
            &state,
            campaign.id,
            &campaign.phone,
            &campaign.country_code,
            &campaign.message,
            campaign.template_name.as_deref(),
            campaign.sent_by_telegram_id,
            campaign.api_key_id,
            admin_telegram_id,
        )
        .await;
    }

    Ok(axum::response::Redirect::temporary(&campaign.target_url))
}

// ---------- Статистика ----------

pub async fn stats(State(state): State<AppState>) -> Result<Json<StatsResponse>, AppError> {
    let admins_count = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM admins")
        .fetch_one(&state.pool)
        .await?;

    let keys_total = sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM api_keys")
        .fetch_one(&state.pool)
        .await?;

    let keys_active = sqlx::query_scalar::<_, i64>(
        "SELECT COUNT(*) FROM api_keys WHERE is_active = TRUE"
    )
    .fetch_one(&state.pool)
    .await?;

    let balance = state
        .sms_client
        .get_balance()
        .await
        .unwrap_or_else(|_| json!({ "balance": "N/A" }));

    Ok(Json(StatsResponse {
        admins_count,
        keys_total,
        keys_active,
        balance,
    }))
}
