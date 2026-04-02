use anyhow::Result;
use axum::http::request::Parts;
use rmcp::{
    ErrorData as McpError,
    handler::server::common::Extension,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::{
    app_state::AuthorizedSession,
    gyazo_api::{
        GyazoUploadImageRequest, delete_image, fetch_image_as_base64, get_image, get_latest_image,
        get_oembed, list_images, search_images, upload_image,
    },
    server::GyazoServer,
};

#[derive(Debug, Deserialize, JsonSchema)]
struct GyazoListImagesArgs {
    page: Option<u32>,
    per_page: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GyazoSearchArgs {
    query: String,
    page: Option<u32>,
    per: Option<u32>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GyazoGetImageArgs {
    image_id: Option<String>,
    image_url: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
struct GyazoOEmbedArgs {
    image_url: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
struct GyazoUploadImageArgs {
    image_data: String,
    access_policy: Option<String>,
    metadata_is_public: Option<bool>,
    referer_url: Option<String>,
    app: Option<String>,
    title: Option<String>,
    description: Option<String>,
    created_at: Option<f64>,
    collection_id: Option<String>,
}

#[rmcp::tool_router(router = gyazo_tool_router, vis = "pub(crate)")]
impl GyazoServer {
    #[rmcp::tool(description = "現在の MCP access token に紐づく Gyazo ユーザーを表示します")]
    fn gyazo_whoami(&self, Extension(parts): Extension<Parts>) -> Result<CallToolResult, McpError> {
        let session = authorized_session(&parts)?;
        let user = session.record.gyazo_user;

        json_result(json!({
            "uid": user.uid,
            "name": user.name,
            "email": user.email,
            "profile_image": user.profile_image,
        }))
    }

    #[rmcp::tool(description = "現在の Gyazo ユーザーがアップロードしたキャプチャを全文検索します")]
    async fn gyazo_search(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(GyazoSearchArgs { query, page, per }): Parameters<GyazoSearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = authorized_session(&parts)?;
        let images = search_images(&session.record.backend_access_token, &query, page, per)
            .await
            .map_err(internal_error)?;

        json_result(images)
    }

    #[rmcp::tool(description = "認証済みユーザーの Gyazo 画像一覧を取得します")]
    async fn gyazo_list_images(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(GyazoListImagesArgs { page, per_page }): Parameters<GyazoListImagesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = authorized_session(&parts)?;
        let images = list_images(&session.record.backend_access_token, page, per_page)
            .await
            .map_err(internal_error)?;

        json_result(images)
    }

    #[rmcp::tool(description = "画像 ID または Gyazo URL を指定して 1 件の画像を取得します")]
    async fn gyazo_get_image(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(args): Parameters<GyazoGetImageArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = authorized_session(&parts)?;
        let image_ref = select_image_ref(args)?;
        let image = get_image(&session.record.backend_access_token, &image_ref)
            .await
            .map_err(internal_error)?;

        json_result(image)
    }

    #[rmcp::tool(description = "画像 ID または Gyazo URL を指定して 1 件の画像を削除します")]
    async fn gyazo_delete_image(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(args): Parameters<GyazoGetImageArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = authorized_session(&parts)?;
        let image_ref = select_image_ref(args)?;
        let deleted = delete_image(&session.record.backend_access_token, &image_ref)
            .await
            .map_err(internal_error)?;

        json_result(deleted)
    }

    #[rmcp::tool(description = "最新の Gyazo 画像を画像本体とメタデータ付きで取得します")]
    async fn gyazo_get_latest_image(
        &self,
        Extension(parts): Extension<Parts>,
    ) -> Result<CallToolResult, McpError> {
        let session = authorized_session(&parts)?;
        let image = get_latest_image(&session.record.backend_access_token)
            .await
            .map_err(internal_error)?;
        let binary = fetch_image_as_base64(&image.url)
            .await
            .map_err(internal_error)?;

        Ok(CallToolResult::success(vec![
            Content::image(binary.data, binary.mime_type),
            Content::text(serde_json::to_string_pretty(&image).map_err(internal_error)?),
        ]))
    }

    #[rmcp::tool(description = "base64 画像を Gyazo にアップロードします")]
    async fn gyazo_upload_image(
        &self,
        Extension(parts): Extension<Parts>,
        Parameters(args): Parameters<GyazoUploadImageArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = authorized_session(&parts)?;
        let uploaded = upload_image(
            &session.record.backend_access_token,
            GyazoUploadImageRequest {
                image_data: args.image_data,
                access_policy: args.access_policy,
                metadata_is_public: args.metadata_is_public,
                referer_url: args.referer_url,
                app: args.app,
                title: args.title,
                description: args.description,
                created_at: args.created_at,
                collection_id: args.collection_id,
            },
        )
        .await
        .map_err(internal_error)?;

        json_result(uploaded)
    }

    #[rmcp::tool(description = "Gyazo 画像ページ URL の oEmbed メタデータを取得します")]
    async fn gyazo_get_oembed_metadata(
        &self,
        Parameters(GyazoOEmbedArgs { image_url }): Parameters<GyazoOEmbedArgs>,
    ) -> Result<CallToolResult, McpError> {
        let oembed = get_oembed(&image_url).await.map_err(internal_error)?;

        json_result(oembed)
    }
}

fn authorized_session(parts: &Parts) -> Result<AuthorizedSession, McpError> {
    parts
        .extensions
        .get::<AuthorizedSession>()
        .cloned()
        .ok_or_else(|| {
            McpError::invalid_params(
                "request context に authorized session が含まれていません",
                None,
            )
        })
}

fn select_image_ref(args: GyazoGetImageArgs) -> Result<String, McpError> {
    match (args.image_id, args.image_url) {
        (Some(image_id), None) if !image_id.trim().is_empty() => Ok(image_id),
        (None, Some(image_url)) if !image_url.trim().is_empty() => Ok(image_url),
        (Some(_), Some(_)) => Err(McpError::invalid_params(
            "image_id と image_url はどちらか一方のみ指定してください",
            None,
        )),
        _ => Err(McpError::invalid_params(
            "image_id か image_url のいずれかを指定してください",
            None,
        )),
    }
}

fn json_result<T: serde::Serialize>(value: T) -> Result<CallToolResult, McpError> {
    let text = serde_json::to_string_pretty(&value).map_err(internal_error)?;
    Ok(CallToolResult::success(vec![Content::text(text)]))
}

fn internal_error(error: impl std::fmt::Display) -> McpError {
    McpError::internal_error(error.to_string(), None)
}
