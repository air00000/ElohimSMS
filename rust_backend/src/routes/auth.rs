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
    hex::encode(hasher.finalize())
}

pub async fn require_api_key(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let key = request
        .headers()
        .get("X-API-Key")
        .and_then(|v| v.to_str().ok())
        .ok_or(AppError::Unauthorized)?;

    // Разрешаем глобальный мастер-ключ из конфигурации.
    if key == state.api_key {
        return Ok(next.run(request).await);
    }

    // Ищем активный ключ в БД по хешу.
    let key_hash = hash_api_key(key);
    let record = sqlx::query_as::<_, (uuid::Uuid,)>(
        "SELECT id FROM api_keys WHERE key_hash = $1 AND is_active = TRUE",
    )
    .bind(&key_hash)
    .fetch_optional(&state.pool)
    .await?;

    if record.is_none() {
        return Err(AppError::Unauthorized);
    }

    // Обновляем last_used_at.
    sqlx::query("UPDATE api_keys SET last_used_at = NOW() WHERE key_hash = $1")
        .bind(&key_hash)
        .execute(&state.pool)
        .await?;

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
        return Err(AppError::Unauthorized);
    }

    Ok(next.run(request).await)
}
