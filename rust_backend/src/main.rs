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
    sms::{
        providers::{DevilTraffProvider, LimitlessTxtProvider, SkyTelecomProvider, SmsMobileCcProvider},
        SmsFailoverClient,
    },
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

fn build_failover_client(config: &Config) -> anyhow::Result<SmsFailoverClient> {
    let mut providers: Vec<Arc<dyn crate::sms::SmsProvider>> = Vec::new();

    for provider_config in &config.sms_providers {
        match provider_config.provider_type {
            crate::sms::ProviderType::DevilTraff => {
                let devil_config = provider_config
                    .devil_traff
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("Devil-Traff config missing for provider {}", provider_config.name))?;
                providers.push(Arc::new(DevilTraffProvider::new(
                    provider_config.name.clone(),
                    devil_config,
                )));
            }
            crate::sms::ProviderType::SkyTelecom => {
                let sky_config = provider_config
                    .sky_telecom
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("SkyTelecom config missing for provider {}", provider_config.name))?;
                providers.push(Arc::new(SkyTelecomProvider::new(
                    provider_config.name.clone(),
                    sky_config,
                )));
            }
            crate::sms::ProviderType::SmsMobileCc => {
                let smsmobile_config = provider_config
                    .smsmobile_cc
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("SMSMobile.cc config missing for provider {}", provider_config.name))?;
                providers.push(Arc::new(SmsMobileCcProvider::new(
                    provider_config.name.clone(),
                    smsmobile_config,
                )));
            }
            crate::sms::ProviderType::LimitlessTxt => {
                let limitless_config = provider_config
                    .limitless_txt
                    .clone()
                    .ok_or_else(|| anyhow::anyhow!("LimitlessTXT config missing for provider {}", provider_config.name))?;
                providers.push(Arc::new(LimitlessTxtProvider::new(
                    provider_config.name.clone(),
                    limitless_config,
                )));
            }
        }
    }

    if providers.is_empty() {
        tracing::warn!("No SMS providers configured; SMS sending will fail");
    } else {
        tracing::info!("Configured {} SMS provider(s)", providers.len());
        for p in &providers {
            tracing::info!(provider = %p.name(), "SMS provider active");
        }
    }

    Ok(SmsFailoverClient::new(providers))
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

    let failover_client = Arc::new(build_failover_client(&config)?);

    let state = AppState::new(pool, &config, failover_client);

    let app = create_router(state);

    let listener = TcpListener::bind(&config.bind_address).await?;
    tracing::info!("Listening on {}", config.bind_address);

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    tracing::info!("Server stopped");
    Ok(())
}
