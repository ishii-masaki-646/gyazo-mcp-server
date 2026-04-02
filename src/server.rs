use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerInfo},
};

#[derive(Clone)]
pub(crate) struct GyazoServer {
    pub(crate) tool_router: ToolRouter<Self>,
}

impl GyazoServer {
    pub(crate) fn new() -> Self {
        Self {
            tool_router: Self::basic_tool_router(),
        }
    }
}

#[rmcp::tool_handler]
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
