use crate::{error::AppError, state::AppState};
use axum::{
    extract::{Request, State},
    middleware::Next,
    response::Response,
};
use sha2::{Digest, Sha256};

pub fn hash_api_key(key: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub async fn require_api_key(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    let hash = hash_api_key(key);

    let key_id: Option<uuid::Uuid> = sqlx::query_scalar(
        "SELECT id FROM api_keys WHERE key_hash = $1 AND is_active = TRUE"
    )
    .bind(&hash)
    .fetch_optional(&state.pool)
    .await?;

    let key_id = key_id.ok_or(AppError::Unauthorized)?;

    sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE id = $1")
        .bind(key_id)
        .execute(&state.pool)
        .await?;

    request.extensions_mut().insert(key_id);

    Ok(next.run(request).await)
}

pub async fn require_internal_bot_token(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = request
        .headers()
        .get("X-Internal-Bot-Token")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    if token != state.internal_bot_token {
        return Err(AppError::Forbidden);
    }

    Ok(next.run(request).await)
}
