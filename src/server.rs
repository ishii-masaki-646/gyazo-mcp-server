use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{Implementation, ServerCapabilities, ServerInfo},
};

use crate::app_state::AppState;

#[derive(Clone)]
pub(crate) struct GyazoServer {
    pub(crate) app_state: Arc<AppState>,
    pub(crate) tool_router: ToolRouter<Self>,
}

impl GyazoServer {
    pub(crate) fn new(app_state: Arc<AppState>) -> Result<Self> {
        Ok(Self {
            app_state,
            tool_router: Self::gyazo_tool_router(),
        })
    }
}

#[rmcp::tool_handler]
impl ServerHandler for GyazoServer {
    fn get_info(&self) -> ServerInfo {
        let has_saved_token = self
            .app_state
            .auth_state_snapshot()
            .map(|state| state.has_saved_oauth_token())
            .unwrap_or(false);

        ServerInfo {
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            instructions: Some(
                if has_saved_token {
                    "Local HTTP MCP server for Gyazo is ready. Available tools: gyazo_whoami, gyazo_list_images, gyazo_get_image, gyazo_get_latest_image, gyazo_upload_image, gyazo_get_oembed_metadata. Saved OAuth token detected."
                } else {
                    "Local HTTP MCP server for Gyazo is ready. Available tools: gyazo_whoami, gyazo_list_images, gyazo_get_image, gyazo_get_latest_image, gyazo_upload_image, gyazo_get_oembed_metadata."
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
