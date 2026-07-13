pub mod auth;
pub mod bot;
pub mod health;
pub mod links;
pub mod sms;
pub mod webhooks;

use crate::{openapi::ApiDoc, state::AppState};
use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::sync::Arc;
use tower_governor::{
    governor::GovernorConfigBuilder, key_extractor::GlobalKeyExtractor, GovernorLayer,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

pub fn create_router(state: AppState) -> Router {
    let public_routes = Router::new()
        .route("/health", get(health::health))
        .route("/r/:short_code", get(bot::redirect))
        .route("/api/v1/links/:short_code", get(links::check_link))
        .route("/api/v1/links/:short_code/verify", post(links::verify_link));

    let governor_conf = Arc::new(
        GovernorConfigBuilder::default()
            .per_second(10)
            .burst_size(50)
            .key_extractor(GlobalKeyExtractor)
            .finish()
            .expect("Failed to build governor config"),
    );

    let api_key_routes = Router::new()
        .route("/sms/send", post(sms::send_sms))
        .route("/webhook", put(webhooks::configure_webhook))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ))
        .layer(GovernorLayer {
            config: governor_conf,
        });

    let bot_routes = Router::new()
        .route("/admin", get(bot::list_admins).post(bot::create_admin))
        .route("/admin/ensure_owner", post(bot::ensure_owner))
        .route("/admin/me/sender_name", post(bot::update_sender_name))
        .route("/admin/:telegram_id", delete(bot::remove_admin))
        .route("/keys", get(bot::list_keys).post(bot::create_key))
        .route("/keys/:id/revoke", post(bot::revoke_key))
        .route("/templates", get(bot::list_templates).post(bot::create_template))
        .route("/templates/:id", delete(bot::delete_template))
        .route("/templates/:id/favorite", post(bot::set_favorite_template))
        .route("/sms/send", post(bot::bot_send_sms))
        .route("/campaigns/send", post(bot::send_campaign))
        .route("/stats", get(bot::stats))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_internal_bot_token,
        ));

    let swagger =
        SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi());

    Router::new()
        .merge(swagger)
        .merge(public_routes)
        .nest("/api/v1", api_key_routes)
        .nest("/bot/v1", bot_routes)
        .with_state(state)
}
