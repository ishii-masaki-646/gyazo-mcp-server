use anyhow::Result;
use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
};
use schemars::JsonSchema;
use serde::Deserialize;

use crate::server::GyazoServer;

#[derive(Debug, Deserialize, JsonSchema)]
struct EchoArgs {
    text: String,
}

#[rmcp::tool_router(router = basic_tool_router, vis = "pub(crate)")]
impl GyazoServer {
    #[rmcp::tool(description = "Check whether the Gyazo MCP server is running")]
    fn ping(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(
            "gyazo-mcp-server is ready",
        )]))
    }

    #[rmcp::tool(
        description = "Show current auth file locations and whether a saved OAuth token exists"
    )]
    fn auth_status(&self) -> Result<CallToolResult, McpError> {
        let auth_state = self
            .app_state
            .auth_state_snapshot()
            .map_err(|error| McpError::internal_error(error.to_string(), None))?;
        let runtime_config = self.app_state.runtime_config();
        let auth_config = self.app_state.auth_config();
        let config_path = auth_state
            .config_file_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(unavailable)".to_string());
        let token_path = auth_state
            .token_file_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(unavailable)".to_string());
        let has_saved_token = auth_state.has_saved_oauth_token();
        let has_oauth_credentials = auth_config.has_oauth_credentials();
        let has_personal_access_token = auth_config.has_personal_access_token();
        let bind_address = runtime_config.bind_address();
        let mcp_url = runtime_config.mcp_url();
        let protected_resource_metadata_root_url =
            runtime_config.protected_resource_metadata_root_url();
        let protected_resource_metadata_url = runtime_config.protected_resource_metadata_url();
        let authorization_server_metadata_url = runtime_config.authorization_server_metadata_url();
        let authorization_endpoint_url = runtime_config.authorization_endpoint_url();
        let token_endpoint_url = runtime_config.token_endpoint_url();
        let oauth_start_url = runtime_config.oauth_start_url();
        let oauth_callback_url = runtime_config.oauth_callback_url();

        Ok(CallToolResult::success(vec![Content::text(format!(
            "bind_address={bind_address}\nmcp_url={mcp_url}\nprotected_resource_metadata_root_url={protected_resource_metadata_root_url}\nprotected_resource_metadata_url={protected_resource_metadata_url}\nauthorization_server_metadata_url={authorization_server_metadata_url}\nauthorization_endpoint_url={authorization_endpoint_url}\ntoken_endpoint_url={token_endpoint_url}\noauth_start_url={oauth_start_url}\noauth_callback_url={oauth_callback_url}\nconfig_file_path={config_path}\ntoken_file_path={token_path}\nhas_saved_oauth_token={has_saved_token}\nhas_oauth_credentials={has_oauth_credentials}\nhas_personal_access_token={has_personal_access_token}"
        ))]))
    }

    #[rmcp::tool(
        description = "Show the local browser URL that starts the Gyazo OAuth login flow"
    )]
    fn oauth_login(&self) -> Result<CallToolResult, McpError> {
        let runtime_config = self.app_state.runtime_config();
        let auth_config = self.app_state.auth_config();

        if !auth_config.has_oauth_credentials() {
            return Err(McpError::invalid_params(
                "GYAZO_MCP_OAUTH_CLIENT_ID と GYAZO_MCP_OAUTH_CLIENT_SECRET を設定してね",
                None,
            ));
        }

        Ok(CallToolResult::success(vec![Content::text(format!(
            "ブラウザで次の URL を開くと Gyazo OAuth login を始められるよ:\n{}",
            runtime_config.oauth_start_url()
        ))]))
    }

    #[rmcp::tool(description = "Echo a string to verify tool invocation over MCP")]
    fn echo(
        &self,
        Parameters(EchoArgs { text }): Parameters<EchoArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}
