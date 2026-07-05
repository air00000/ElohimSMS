pub mod admin;
pub mod auth;
pub mod campaigns;
pub mod health;
pub mod keys;
pub mod links;
pub mod sms;
pub mod templates;

use crate::{openapi::ApiDoc, state::AppState};
use axum::{
    middleware,
    routing::{get, post},
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
        .route("/api/v1/links/:short_code", get(links::get_link_info))
        .route("/api/v1/links/:short_code/verify", post(links::verify_captcha));

    // Rate limiting: 10 запросов в секунду с burst до 50 (глобально)
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
        .route("/sms/balance", get(sms::get_balance))
        .route("/sms/routes", get(sms::get_routes))
        .route("/campaigns/send", post(campaigns::send_campaign))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::require_api_key,
        ))
        .layer(GovernorLayer {
            config: governor_conf,
        });

    let bot_routes = Router::new()
        .route("/admin", get(admin::list_admins).post(admin::create_admin))
        .route("/admin/:telegram_id", get(admin::remove_admin))
        .route("/keys", get(keys::list_keys).post(keys::create_key))
        .route("/keys/:id/revoke", post(keys::revoke_key))
        .route("/templates", get(templates::list).post(templates::create_or_update))
        .route("/templates/:country_code", get(templates::remove))
        .route("/campaigns/send", post(campaigns::send_campaign_as_bot))
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
