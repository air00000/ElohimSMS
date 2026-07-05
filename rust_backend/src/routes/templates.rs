use crate::{
    error::AppError,
    models::{CreateTemplateRequest, Template},
    state::AppState,
    templates::{delete_template, list_templates, upsert_template},
};
use axum::{extract::State, Json};

#[utoipa::path(
    get,
    path = "/bot/v1/templates",
    responses(
        (status = 200, description = "List of templates", body = [Template]),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("internal_bot_token" = [])
    )
)]
pub async fn list(State(state): State<AppState>) -> Result<Json<Vec<Template>>, AppError> {
    let templates = list_templates(&state.pool).await?;
    Ok(Json(templates))
}

#[utoipa::path(
    post,
    path = "/bot/v1/templates",
    request_body = CreateTemplateRequest,
    responses(
        (status = 200, description = "Template created/updated", body = Template),
        (status = 400, description = "Bad request"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("internal_bot_token" = [])
    )
)]
pub async fn create_or_update(
    State(state): State<AppState>,
    Json(payload): Json<CreateTemplateRequest>,
) -> Result<Json<Template>, AppError> {
    let template = upsert_template(&state.pool, &payload.country_code, &payload.text).await?;
    Ok(Json(template))
}

#[utoipa::path(
    delete,
    path = "/bot/v1/templates/{country_code}",
    params(
        ("country_code" = String, Path, description = "ISO country code")
    ),
    responses(
        (status = 204, description = "Template deleted"),
        (status = 401, description = "Unauthorized"),
        (status = 403, description = "Forbidden"),
        (status = 404, description = "Not found")
    ),
    security(
        ("internal_bot_token" = [])
    )
)]
pub async fn remove(
    State(state): State<AppState>,
    axum::extract::Path(country_code): axum::extract::Path<String>,
) -> Result<axum::http::StatusCode, AppError> {
    delete_template(&state.pool, &country_code).await?;
    Ok(axum::http::StatusCode::NO_CONTENT)
}
