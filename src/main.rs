use anyhow::Result;
use rmcp::{
    ErrorData as McpError, ServerHandler, ServiceExt,
    handler::server::{router::tool::ToolRouter, wrapper::Parameters},
    model::*,
    tool, tool_handler, tool_router,
    transport::stdio,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing_subscriber::EnvFilter;

#[derive(Debug, Deserialize, JsonSchema)]
struct EchoArgs {
    text: String,
}

#[derive(Clone)]
struct GyazoServer {
    tool_router: ToolRouter<Self>,
}

impl GyazoServer {
    fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }
}

#[tool_router]
impl GyazoServer {
    #[tool(description = "Check whether the Gyazo MCP server is running")]
    fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "gyazo-mcp-server is ready",
        )]))
    }

    #[tool(description = "Echo a string to verify tool invocation over MCP")]
    fn echo(
        &self,
        Parameters(EchoArgs { text }): Parameters<EchoArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}

#[tool_handler]
impl ServerHandler for GyazoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                "Minimal RMCP bootstrap for the Gyazo MCP server. Available tools: ping, echo."
                    .to_string(),
            ),
            server_info: Implementation {
                name: env!("CARGO_PKG_NAME").into(),
                title: None,
                version: env!("CARGO_PKG_VERSION").into(),
                icons: None,
                website_url: None,
            },
            ..Default::default()
        }
    }
}

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
