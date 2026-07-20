use crate::{models::HealthResponse, state::AppState};
use axum::{extract::State, Json};

/// Проверка состояния сервиса
///
/// Возвращает состояние сервиса и доступность базы данных.
/// Аутентификация не требуется.
#[utoipa::path(
    get,
    path = "/health",
    tag = "health",
    responses(
        (status = 200, description = "Сервис работает", body = HealthResponse)
    ),
    security(())
)]
pub async fn health(State(state): State<AppState>) -> Json<HealthResponse> {
    let db_status = match sqlx::query("SELECT 1").fetch_one(&state.pool).await {
        Ok(_) => "ok",
        Err(e) => {
            tracing::error!("Health check database query failed: {}", e);
            "error"
        }
    };

    Json(HealthResponse {
        status: "ok".to_string(),
        database: db_status.to_string(),
        timestamp: chrono::Utc::now().to_rfc3339(),
    })
}
