use crate::{
    error::AppError,
    routes::{
        bot::notify_admin,
        webhooks::dispatch_link_verified,
    },
    state::AppState,
};
use axum::{extract::Path, extract::State, Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Serialize)]
pub struct LinkVerifyResponse {
    pub target_url: String,
}

#[derive(Debug, Deserialize)]
pub struct LinkVerifyRequest {
    pub g_recaptcha_response: String,
}

#[derive(Debug, FromRow)]
struct CampaignInfo {
    id: Uuid,
    short_code: String,
    target_url: String,
    phone: String,
    country_code: String,
    message: String,
    template_name: Option<String>,
    sent_by_telegram_id: Option<i64>,
    api_key_id: Option<Uuid>,
    external_id: Option<String>,
}

async fn get_owner_telegram_id(pool: &sqlx::PgPool) -> Result<Option<i64>, AppError> {
    let owner = sqlx::query_scalar::<_, Option<i64>>(
        "SELECT telegram_id FROM admins WHERE is_owner = TRUE LIMIT 1",
    )
    .fetch_optional(pool)
    .await?;
    Ok(owner.flatten())
}

async fn verify_recaptcha(secret: &str, response: &str) -> anyhow::Result<bool> {
    if secret.is_empty() {
        tracing::warn!("RECAPTCHA_SECRET is not set; skipping verification");
        return Ok(true);
    }

    let client = reqwest::Client::new();
    let result = client
        .post("https://www.google.com/recaptcha/api/siteverify")
        .form(&[("secret", secret), ("response", response)])
        .send()
        .await?;

    let status = result.status();
    let body = result.json::<Value>().await?;
    tracing::debug!(status = %status, body = %body, "reCAPTCHA verification response");

    Ok(body
        .get("success")
        .and_then(|v| v.as_bool())
        .unwrap_or(false))
}

/// Проверяет существование короткой ссылки.
///
/// Frontend (captcha_site_example) вызывает этот endpoint перед показом капчи.
pub async fn check_link(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
) -> Result<Json<Value>, AppError> {
    tracing::info!(short_code = %short_code, "Checking link existence");

    let exists = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS(SELECT 1 FROM campaigns WHERE short_code = $1)",
    )
    .bind(&short_code)
    .fetch_one(&state.pool)
    .await?;

    if exists {
        Ok(Json(json!({ "exists": true })))
    } else {
        tracing::warn!(short_code = %short_code, "Link not found");
        Err(AppError::NotFound)
    }
}

/// Верифицирует капчу и возвращает целевой URL.
///
/// Вызывается frontend после успешного прохождения reCAPTCHA.
/// Также фиксирует переход (click_count + campaign_clicks) и шлёт уведомление в Telegram.
pub async fn verify_link(
    State(state): State<AppState>,
    Path(short_code): Path<String>,
    Json(payload): Json<LinkVerifyRequest>,
) -> Result<Json<LinkVerifyResponse>, AppError> {
    tracing::info!(short_code = %short_code, "Verifying link captcha");

    let recaptcha_ok = verify_recaptcha(&state.config.recaptcha_secret, &payload.g_recaptcha_response)
        .await
        .map_err(|e| AppError::Internal(format!("reCAPTCHA verification failed: {e}")))?;

    if !recaptcha_ok {
        tracing::warn!(short_code = %short_code, "reCAPTCHA verification failed");
        return Err(AppError::BadRequest("Invalid captcha".to_string()));
    }

    let campaign = sqlx::query_as::<_, CampaignInfo>(
        "SELECT id, short_code, target_url, phone, country_code, message, template_name, sent_by_telegram_id, api_key_id, external_id
         FROM campaigns WHERE short_code = $1"
    )
    .bind(&short_code)
    .fetch_optional(&state.pool)
    .await?;

    let campaign = match campaign {
        Some(c) => c,
        None => {
            tracing::warn!(short_code = %short_code, "Campaign not found after captcha");
            return Err(AppError::NotFound);
        }
    };

    // Определяем, кому слать уведомление.
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

    if admin_telegram_id.is_none() {
        admin_telegram_id = get_owner_telegram_id(&state.pool).await?;
    }

    tracing::info!(
        short_code = %short_code,
        campaign_id = %campaign.id,
        admin_telegram_id = ?admin_telegram_id,
        "Resolved admin for click notification"
    );

    // Фиксируем переход.
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

    tracing::info!(
        short_code = %short_code,
        campaign_id = %campaign.id,
        is_first_click,
        "Click counter updated"
    );

    sqlx::query("INSERT INTO campaign_clicks (campaign_id) VALUES ($1)")
        .bind(campaign.id)
        .execute(&state.pool)
        .await?;

    if is_first_click {
        if let Some(tid) = admin_telegram_id {
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
            tracing::info!(
                short_code = %short_code,
                campaign_id = %campaign.id,
                telegram_id = tid,
                "Calling notify_admin"
            );
            notify_admin(&state, tid, text);
        }
    }

    if is_first_click {
        if let Some(api_key_id) = campaign.api_key_id {
            let webhook_state = state.clone();
            let campaign_id = campaign.id;
            let external_id = campaign.external_id.clone();
            let short_code = campaign.short_code.clone();
            let occurred_at = chrono::Utc::now();

            // Не задерживаем редирект клиента ожиданием внешнего сервиса.
            tokio::spawn(async move {
                if let Err(error) = dispatch_link_verified(
                    &webhook_state,
                    api_key_id,
                    campaign_id,
                    external_id,
                    short_code,
                    1,
                    occurred_at,
                )
                .await
                {
                    tracing::error!(
                        campaign_id = %campaign_id,
                        api_key_id = %api_key_id,
                        error = %error,
                        "Failed to deliver campaign.link_verified webhook"
                    );
                }
            });
        }
    }

    tracing::info!(
        short_code = %short_code,
        campaign_id = %campaign.id,
        "Link verified, redirecting to target URL"
    );

    Ok(Json(LinkVerifyResponse {
        target_url: campaign.target_url,
    }))
}
