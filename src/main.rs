mod server;
mod tools;

use anyhow::Result;
use rmcp::{ServiceExt, transport::stdio};
use tracing_subscriber::EnvFilter;

use crate::server::GyazoServer;

#[tokio::main]
async fn main() -> Result<()> {
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
