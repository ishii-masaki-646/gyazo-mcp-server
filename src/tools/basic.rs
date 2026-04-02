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

    #[rmcp::tool(description = "Echo a string to verify tool invocation over MCP")]
    fn echo(
        &self,
        Parameters(EchoArgs { text }): Parameters<EchoArgs>,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(text)]))
    }
}
