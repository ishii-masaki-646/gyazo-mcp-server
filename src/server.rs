use anyhow::Result;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerInfo},
};

use crate::auth::state::AuthState;
use crate::runtime_config::RuntimeConfig;

#[derive(Clone)]
pub(crate) struct GyazoServer {
    pub(crate) auth_state: AuthState,
    pub(crate) runtime_config: RuntimeConfig,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl GyazoServer {
    pub(crate) fn new(runtime_config: RuntimeConfig) -> Result<Self> {
        Ok(Self {
            auth_state: AuthState::load()?,
            runtime_config,
            tool_router: Self::basic_tool_router(),
        })
    }
}

#[rmcp::tool_handler]
impl ServerHandler for GyazoServer {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                if self.auth_state.has_saved_oauth_token() {
                    "Local HTTP MCP server for Gyazo is ready. Available tools: ping, auth_status, echo. Saved OAuth token detected."
                } else {
                    "Local HTTP MCP server for Gyazo is ready. Available tools: ping, auth_status, echo."
                }
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
