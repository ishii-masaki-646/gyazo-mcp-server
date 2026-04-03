use std::sync::Arc;

use anyhow::Result;
use axum::http::request::Parts;
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
    app_state::{AppState, AuthorizedSession},
    gyazo_api::{
        create_image_resource_uri, extract_image_id_from_resource_uri, fetch_image_as_base64,
        format_image_metadata_markdown, get_image, list_images,
    },
    mcp_oauth::authorized_session_from_parts,
};

#[derive(Clone)]
pub(crate) struct GyazoServer {
    pub(crate) app_state: Arc<AppState>,
    pub(crate) tool_router: ToolRouter<Self>,
    fallback_authorized_session: Option<AuthorizedSession>,
}

impl GyazoServer {
    pub(crate) fn new(app_state: Arc<AppState>) -> Result<Self> {
        Self::build(app_state, None)
    }

    pub(crate) fn with_fallback_authorized_session(
        app_state: Arc<AppState>,
        authorized_session: AuthorizedSession,
    ) -> Result<Self> {
        Self::build(app_state, Some(authorized_session))
    }

    fn build(
        app_state: Arc<AppState>,
        fallback_authorized_session: Option<AuthorizedSession>,
    ) -> Result<Self> {
        Ok(Self {
            app_state,
            tool_router: Self::gyazo_tool_router(),
            fallback_authorized_session,
        })
    }

    pub(crate) fn authorized_session_for_request(
        &self,
        context: &RequestContext<rmcp::service::RoleServer>,
    ) -> Result<AuthorizedSession, rmcp::ErrorData> {
        authorized_session_from_context(
            &self.app_state,
            context,
            self.fallback_authorized_session.as_ref(),
        )
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
                    "Gyazo 向けのローカル HTTP MCP サーバーは利用可能です。利用可能な tools は gyazo_whoami、gyazo_search、gyazo_list_images、gyazo_get_image、gyazo_delete_image、gyazo_get_latest_image、gyazo_upload_image、gyazo_get_oembed_metadata です。Resources は gyazo-mcp:///image_id 形式で利用できます。保存済みの OAuth token を検出しました。"
                } else {
                    "Gyazo 向けのローカル HTTP MCP サーバーは利用可能です。利用可能な tools は gyazo_whoami、gyazo_search、gyazo_list_images、gyazo_get_image、gyazo_delete_image、gyazo_get_latest_image、gyazo_upload_image、gyazo_get_oembed_metadata です。Resources は gyazo-mcp:///image_id 形式で利用できます。"
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
        context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ListResourcesResult, rmcp::ErrorData> {
        let session = self.authorized_session_for_request(&context)?;
        let listed = list_images(&session.record.backend_access_token, Some(1), Some(20))
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
                        name: image
                            .metadata
                            .title
                            .unwrap_or_else(|| image.image_id.clone()),
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
        context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<ReadResourceResult, rmcp::ErrorData> {
        let session = self.authorized_session_for_request(&context)?;
        let image_id = extract_image_id_from_resource_uri(&request.uri).map_err(internal_error)?;
        let image = get_image(&session.record.backend_access_token, &image_id)
            .await
            .map_err(internal_error)?;
        let image_url = image
            .url
            .as_deref()
            .or(image.thumb_url.as_deref())
            .ok_or_else(|| {
                internal_error("Gyazo image detail に利用可能な画像 URL が含まれていません")
            })?;
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

fn authorized_session_from_context(
    app_state: &AppState,
    context: &RequestContext<rmcp::service::RoleServer>,
    fallback_authorized_session: Option<&AuthorizedSession>,
) -> Result<AuthorizedSession, rmcp::ErrorData> {
    if let Some(session) = context.extensions.get::<AuthorizedSession>().cloned() {
        return Ok(session);
    }

    let Some(parts) = context.extensions.get::<Parts>() else {
        return fallback_authorized_session.cloned().ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                "request context に request parts が含まれていません",
                None,
            )
        });
    };

    authorized_session_from_parts(app_state, parts)
        .map_err(internal_error)?
        .or_else(|| fallback_authorized_session.cloned())
        .ok_or_else(|| {
            rmcp::ErrorData::invalid_params(
                "request context に authorized session が含まれていません",
                None,
            )
        })
}

fn internal_error(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(error.to_string(), None)
}
