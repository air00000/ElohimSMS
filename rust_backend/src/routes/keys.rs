use crate::{
    error::AppError,
    models::{ApiKey, CreateApiKeyRequest, CreateApiKeyResponse},
    pagination::{PaginatedResponse, PaginationParams},
    routes::auth::hash_api_key,
    state::AppState,
};
use axum::{
    extract::{Query, State},
    Json,
};
use rand::Rng;

fn generate_api_key() -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";
    let mut rng = rand::thread_rng();
    (0..48)
        .map(|_| CHARSET[rng.gen_range(0..CHARSET.len())] as char)
        .collect()
}

#[utoipa::path(
    get,
    path = "/bot/v1/keys",
    params(PaginationParams),
    responses(
        (status = 200, description = "List of API keys", body = PaginatedResponse<ApiKey>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_keys(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<ApiKey>>, AppError> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM api_keys")
        .fetch_one(&state.pool)
        .await?;

    let keys = sqlx::query_as::<_, ApiKey>(
        "SELECT * FROM api_keys ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(PaginatedResponse::new(
        keys,
        params.page,
        params.limit(),
        total,
    )))
}

#[utoipa::path(
    post,
    path = "/bot/v1/keys",
    request_body = CreateApiKeyRequest,
    responses(
        (status = 200, description = "API key created", body = CreateApiKeyResponse),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn create_key(
    State(state): State<AppState>,
    Json(payload): Json<CreateApiKeyRequest>,
) -> Result<Json<CreateApiKeyResponse>, AppError> {
    if payload.name.trim().is_empty() {
        return Err(AppError::BadRequest("Key name cannot be empty".to_string()));
    }

    let key = generate_api_key();
    let key_hash = hash_api_key(&key);

    let record = sqlx::query_as::<_, ApiKey>(
        "INSERT INTO api_keys (key_hash, name, created_by_telegram_id) VALUES ($1, $2, $3) RETURNING *"
    )
    .bind(&key_hash)
    .bind(&payload.name)
    .bind(payload.created_by_telegram_id)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(CreateApiKeyResponse {
        id: record.id,
        key,
        name: record.name,
    }))
}

#[utoipa::path(
    post,
    path = "/bot/v1/keys/{id}/revoke",
    params(
        ("id" = Uuid, Path, description = "API key ID")
    ),
    responses(
        (status = 204, description = "API key revoked"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn revoke_key(
    State(state): State<AppState>,
    axum::extract::Path(id): axum::extract::Path<uuid::Uuid>,
) -> Result<axum::http::StatusCode, AppError> {
    let result = sqlx::query("UPDATE api_keys SET is_active = FALSE WHERE id = $1")
        .bind(id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}
