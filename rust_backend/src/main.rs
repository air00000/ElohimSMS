mod config;
mod db;
mod error;
mod models;
mod openapi;
mod phone;
mod routes;
mod sms;
mod state;

use crate::{
    config::Config,
    db::init_db,
    routes::create_router,
    sms::{SmsClient, SmsGatewayConfig},
    state::AppState,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    tracing::info!("Shutdown signal received, starting graceful shutdown...");
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let env_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| "elohim_sms_backend=debug,tower_http=debug".into());

    let json_logs = std::env::var("JSON_LOGS")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if json_logs {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(env_filter)
            .with(tracing_subscriber::fmt::layer())
            .init();
    }

    let config = Config::from_env()?;
    tracing::info!("Starting ElohimSMS backend on {}", config.bind_address);

    let pool = init_db(&config.database_url).await?;
    tracing::info!("Database connected and migrations applied");

    let sms_config = SmsGatewayConfig::from(&config);
    let sms_client = Arc::new(SmsClient::new(sms_config));

    let state = AppState::new(pool, &config, sms_client);

    let app = create_router(state);

    let listener = TcpListener::bind(&config.bind_address).await?;
    tracing::info!("Listening on {}", config.bind_address);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}
