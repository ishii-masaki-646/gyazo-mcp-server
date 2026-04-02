mod server;
mod tools;

use std::{env, path::PathBuf};

use anyhow::Result;
use dotenvy::{dotenv, from_path};
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

use crate::server::GyazoServer;

fn config_dir_env_path() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/gyazo-mcp-server/.env"))
}

fn load_env_files() -> Result<()> {
    if let Some(path) = config_dir_env_path()
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

    let service = GyazoServer::new().serve(stdio()).await?;
    service.waiting().await?;
    Ok(())
}
