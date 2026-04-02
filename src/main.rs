mod auth;
mod runtime_config;
mod server;
mod tools;

use std::{io, sync::Arc};

use anyhow::Result;
use axum::{Router, routing::get};
use dotenvy::{dotenv, from_path};
use rmcp::transport::{
    StreamableHttpServerConfig, StreamableHttpService,
    streamable_http_server::session::local::LocalSessionManager,
};
use tracing_subscriber::EnvFilter;

use crate::auth::paths;
use crate::runtime_config::RuntimeConfig;
use crate::server::GyazoServer;

fn load_env_files() -> Result<()> {
    if let Some(path) = paths::env_file_path()
        && path.exists()
    {
        from_path(path)?;
    }

    if let Err(error) = dotenv()
        && !error.not_found()
    {
        return Err(error.into());
    }

    Ok(())
}

async fn oauth_callback_handler() -> &'static str {
    "OAuth callback endpoint is ready. Login flow implementation will be added next."
}

async fn root_handler() -> &'static str {
    "gyazo-mcp-server is running"
}

#[tokio::main]
async fn main() -> Result<()> {
    load_env_files()?;

    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("gyazo_mcp_server=info,rmcp=info")),
        )
        .with_writer(std::io::stderr)
        .init();

    let runtime_config = RuntimeConfig::from_env()?;
    let service_runtime_config = runtime_config.clone();
    let service: StreamableHttpService<GyazoServer, LocalSessionManager> =
        StreamableHttpService::new(
            move || GyazoServer::new(service_runtime_config.clone()).map_err(io::Error::other),
            Arc::new(LocalSessionManager::default()),
            StreamableHttpServerConfig::default(),
        );

    let app = Router::new()
        .route("/", get(root_handler))
        .route(
            runtime_config.oauth_callback_path(),
            get(oauth_callback_handler),
        )
        .nest_service(runtime_config.mcp_path(), service);

    let listener = tokio::net::TcpListener::bind(runtime_config.bind_address()).await?;
    tracing::info!(
        bind_address = %runtime_config.bind_address(),
        mcp_url = %runtime_config.mcp_url(),
        oauth_callback_url = %runtime_config.oauth_callback_url(),
        "starting gyazo mcp http server",
    );

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await?;

    Ok(())
}
