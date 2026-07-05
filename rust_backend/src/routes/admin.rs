use crate::{
    error::AppError,
    models::{Admin, CreateAdminRequest},
    pagination::{PaginatedResponse, PaginationParams},
    state::AppState,
};
use axum::{
    extract::{Query, State},
    Json,
};

#[utoipa::path(
    get,
    path = "/bot/v1/admin",
    params(PaginationParams),
    responses(
        (status = 200, description = "List of admins", body = PaginatedResponse<Admin>),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn list_admins(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Admin>>, AppError> {
    let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM admins")
        .fetch_one(&state.pool)
        .await?;

    let admins = sqlx::query_as::<_, Admin>(
        "SELECT * FROM admins ORDER BY created_at DESC LIMIT $1 OFFSET $2"
    )
    .bind(params.limit())
    .bind(params.offset())
    .fetch_all(&state.pool)
    .await?;

    Ok(Json(PaginatedResponse::new(
        admins,
        params.page,
        params.limit(),
        total,
    )))
}

#[utoipa::path(
    post,
    path = "/bot/v1/admin",
    request_body = CreateAdminRequest,
    responses(
        (status = 200, description = "Admin created", body = Admin),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    )
)]
pub async fn create_admin(
    State(state): State<AppState>,
    Json(payload): Json<CreateAdminRequest>,
) -> Result<Json<Admin>, AppError> {
    let admin = sqlx::query_as::<_, Admin>(
        r#"
        INSERT INTO admins (telegram_id, username, is_owner)
        VALUES ($1, $2, FALSE)
        ON CONFLICT (telegram_id) DO UPDATE SET username = EXCLUDED.username
        RETURNING *
        "#,
    )
    .bind(payload.telegram_id)
    .bind(payload.username)
    .fetch_one(&state.pool)
    .await?;

    Ok(Json(admin))
}

#[utoipa::path(
    delete,
    path = "/bot/v1/admin/{telegram_id}",
    params(
        ("telegram_id" = i64, Path, description = "Telegram ID of admin to remove")
    ),
    responses(
        (status = 204, description = "Admin removed"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    )
)]
pub async fn remove_admin(
    State(state): State<AppState>,
    axum::extract::Path(telegram_id): axum::extract::Path<i64>,
) -> Result<axum::http::StatusCode, AppError> {
    let result = sqlx::query("DELETE FROM admins WHERE telegram_id = $1 AND is_owner = FALSE")
        .bind(telegram_id)
        .execute(&state.pool)
        .await?;

    if result.rows_affected() == 0 {
        return Err(AppError::NotFound);
    }

    Ok(axum::http::StatusCode::NO_CONTENT)
}
