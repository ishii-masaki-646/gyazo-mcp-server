use std::sync::Arc;

use anyhow::Result;
use rmcp::{
    ServerHandler,
    handler::server::router::tool::ToolRouter,
    model::{
        AnnotateAble, Implementation, ListResourcesResult, PaginatedRequestParam, RawResource,
        ReadResourceRequestParam, ReadResourceResult, ResourceContents, ServerCapabilities,
        ServerInfo,
    },
    service::RequestContext,
};

use crate::{
    app_state::AppState,
    gyazo_api::{
        create_image_resource_uri, extract_image_id_from_resource_uri,
        fetch_image_as_base64, format_image_metadata_markdown, get_image, list_images,
    },
};

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
            capabilities: ServerCapabilities::builder()
                .enable_tools()
                .enable_resources()
                .build(),
            instructions: Some(
                if has_saved_token {
                    "Local HTTP MCP server for Gyazo is ready. Available tools: gyazo_whoami, gyazo_search, gyazo_list_images, gyazo_get_image, gyazo_delete_image, gyazo_get_latest_image, gyazo_upload_image, gyazo_get_oembed_metadata. Resources are available as gyazo-mcp:///image_id. Saved OAuth token detected."
                } else {
                    "Local HTTP MCP server for Gyazo is ready. Available tools: gyazo_whoami, gyazo_search, gyazo_list_images, gyazo_get_image, gyazo_delete_image, gyazo_get_latest_image, gyazo_upload_image, gyazo_get_oembed_metadata. Resources are available as gyazo-mcp:///image_id."
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

    async fn list_resources(
        &self,
        _request: Option<PaginatedRequestParam>,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let backend_access_token = backend_access_token(&self.app_state)?;
        let listed = list_images(&backend_access_token, Some(1), Some(20))
            .await
            .map_err(internal_error)?;

        Ok(ListResourcesResult {
            resources: listed
                .images
                .into_iter()
                .filter(|image| !image.image_id.trim().is_empty())
                .map(|image| {
                    RawResource {
                        uri: create_image_resource_uri(&image.image_id),
                        name: image.metadata.title.unwrap_or_else(|| image.image_id.clone()),
                        title: None,
                        description: None,
                        mime_type: Some(format!("image/{}", image.image_type)),
                        size: None,
                        icons: None,
                    }
                    .no_annotation()
                })
                .collect(),
            next_cursor: None,
        })
    }

    async fn read_resource(
        &self,
        request: ReadResourceRequestParam,
        _context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        let backend_access_token = backend_access_token(&self.app_state)?;
        let image_id = extract_image_id_from_resource_uri(&request.uri).map_err(internal_error)?;
        let image = get_image(&backend_access_token, &image_id)
            .await
            .map_err(internal_error)?;
        let image_url = image
            .url
            .as_deref()
            .or(image.thumb_url.as_deref())
            .ok_or_else(|| internal_error("Gyazo image detail did not include a usable image URL"))?;
        let image_binary = fetch_image_as_base64(image_url)
            .await
            .map_err(internal_error)?;
        let metadata_markdown = format_image_metadata_markdown(&image);

        Ok(ReadResourceResult {
            contents: vec![
                ResourceContents::BlobResourceContents {
                    uri: request.uri.clone(),
                    mime_type: Some(image_binary.mime_type),
                    blob: image_binary.data,
                    meta: None,
                },
                ResourceContents::TextResourceContents {
                    uri: request.uri,
                    mime_type: Some("text/plain".to_string()),
                    text: metadata_markdown,
                    meta: None,
                },
            ],
        })
    }
}

fn backend_access_token(app_state: &AppState) -> Result<String, rmcp::ErrorData> {
    app_state
        .resolve_backend_access_token()
        .map_err(internal_error)?
        .ok_or_else(|| {
            rmcp::ErrorData::invalid_params("missing backend access token", None)
        })
}

fn internal_error(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(error.to_string(), None)
}
