use crate::{
    error::AppError,
    links::{get_campaign_by_short_code, increment_click_count, mark_verified},
    models::{LinkInfoResponse, VerifyCaptchaRequest, VerifyCaptchaResponse},
    state::AppState,
};
use axum::{extract::State, Json};

#[utoipa::path(
    get,
    path = "/api/v1/links/{short_code}",
    params(
        ("short_code" = String, Path, description = "Short link code")
    ),
    responses(
        (status = 200, description = "Link info", body = LinkInfoResponse),
        (status = 404, description = "Not found")
    ),
    security(())
)]
pub async fn get_link_info(
    State(state): State<AppState>,
    axum::extract::Path(short_code): axum::extract::Path<String>,
) -> Result<Json<LinkInfoResponse>, AppError> {
    let campaign = get_campaign_by_short_code(&state.pool, &short_code)
        .await?
        .ok_or(AppError::NotFound)?;

    increment_click_count(&state.pool, campaign.id).await?;

    Ok(Json(LinkInfoResponse {
        short_code: campaign.short_code,
        created_at: campaign.created_at,
    }))
}

#[utoipa::path(
    post,
    path = "/api/v1/links/{short_code}/verify",
    params(
        ("short_code" = String, Path, description = "Short link code")
    ),
    request_body = VerifyCaptchaRequest,
    responses(
        (status = 200, description = "Captcha verified, target URL returned", body = VerifyCaptchaResponse),
        (status = 400, description = "Bad request"),
        (status = 403, description = "Forbidden / captcha failed"),
        (status = 404, description = "Not found")
    ),
    security(())
)]
pub async fn verify_captcha(
    State(state): State<AppState>,
    axum::extract::Path(short_code): axum::extract::Path<String>,
    Json(payload): Json<VerifyCaptchaRequest>,
) -> Result<Json<VerifyCaptchaResponse>, AppError> {
    let campaign = get_campaign_by_short_code(&state.pool, &short_code)
        .await?
        .ok_or(AppError::NotFound)?;

    state
        .captcha_verifier
        .verify(&payload.g_recaptcha_response)
        .await?;

    mark_verified(&state.pool, campaign.id).await?;

    Ok(Json(VerifyCaptchaResponse {
        target_url: campaign.target_url,
    }))
}
