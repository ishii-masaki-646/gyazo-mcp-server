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
        let config_path = self
            .auth_state
            .config_file_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(unavailable)".to_string());
        let token_path = self
            .auth_state
            .token_file_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "(unavailable)".to_string());
        let has_saved_token = self.auth_state.has_saved_oauth_token();
        let bind_address = self.runtime_config.bind_address();
        let mcp_url = self.runtime_config.mcp_url();
        let oauth_callback_url = self.runtime_config.oauth_callback_url();

        Ok(CallToolResult::success(vec![Content::text(format!(
            "bind_address={bind_address}\nmcp_url={mcp_url}\noauth_callback_url={oauth_callback_url}\nconfig_file_path={config_path}\ntoken_file_path={token_path}\nhas_saved_oauth_token={has_saved_token}"
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
