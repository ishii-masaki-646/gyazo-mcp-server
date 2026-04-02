mod auth;
mod server;
mod tools;

use anyhow::Result;
use dotenvy::{dotenv, from_path};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

use crate::auth::paths;
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

    let service = GyazoServer::new()?.serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
