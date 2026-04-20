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
    mcp_oauth::{authorized_session_from_parts, get_verified_session},
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

    pub(crate) async fn authorized_session_for_request(
        &self,
        context: &RequestContext<rmcp::service::RoleServer>,
    ) -> Result<AuthorizedSession, rmcp::ErrorData> {
        authorized_session_from_context(
            &self.app_state,
            context,
            self.fallback_authorized_session.as_ref(),
        )
        .await
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
        let session = self.authorized_session_for_request(&context).await?;
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
                            .as_ref()
                            .and_then(|m| m.title.clone())
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
        let session = self.authorized_session_for_request(&context).await?;
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

async fn authorized_session_from_context(
    app_state: &AppState,
    context: &RequestContext<rmcp::service::RoleServer>,
    fallback_authorized_session: Option<&AuthorizedSession>,
) -> Result<AuthorizedSession, rmcp::ErrorData> {
    resolve_authorized_session(app_state, &context.extensions, fallback_authorized_session).await
}

/// `authorized_session_from_context` のコアロジック。`RequestContext` から
/// `Extensions` を剥がして渡す形にすることで、`Peer` 等を構築せずに
/// 単体テストできるようにしている。
async fn resolve_authorized_session(
    app_state: &AppState,
    extensions: &rmcp::model::Extensions,
    fallback_authorized_session: Option<&AuthorizedSession>,
) -> Result<AuthorizedSession, rmcp::ErrorData> {
    // 1. middleware が直接 `AuthorizedSession` を extensions に挿入していたら
    //    それを使う。現状 rmcp の StreamableHttpService は任意の extension を
    //    tool handler の RequestContext へ転送しないため、このパスは (将来
    //    rmcp 側で対応された場合の) 備えとして残している。
    if let Some(session) = extensions.get::<AuthorizedSession>().cloned() {
        return Ok(session);
    }

    // 2. Parts (HTTP request の headers 等) が forwarded されていれば、
    //    Authorization ヘッダから Bearer token を抽出して検証する。
    //    stdio transport では Parts が存在しないので fallback session を使う。
    if let Some(parts) = extensions.get::<Parts>() {
        if let Some(session) =
            authorized_session_from_parts(app_state, parts).map_err(internal_error)?
        {
            return Ok(session);
        }
    } else if let Some(session) = fallback_authorized_session.cloned() {
        return Ok(session);
    }

    // 3. Workaround: anthropics/claude-code#46879
    //    Authorization ヘッダを送ってこない Claude Code 互換のため、
    //    middleware と同じく `get_verified_session` の結果を最終 fallback に
    //    する。middleware は `request.extensions_mut()` に session を挿入して
    //    いるが、rmcp がそれを tool handler の context まで転送しないため
    //    ここで再取得する。キャッシュが効くので Gyazo API への再問い合わせは
    //    通常発生しない。
    if let Some(session) = get_verified_session(app_state).await {
        return Ok(session);
    }

    // 4. Parts 経由でも get_verified_session でも取れず、fallback もなければ
    //    明確にエラー。
    fallback_authorized_session.cloned().ok_or_else(|| {
        rmcp::ErrorData::invalid_params(
            "request context に authorized session が含まれていません",
            None,
        )
    })
}

fn internal_error(error: impl std::fmt::Display) -> rmcp::ErrorData {
    rmcp::ErrorData::internal_error(error.to_string(), None)
}

#[cfg(test)]
mod tests {
    use std::time::Instant;

    use rmcp::model::Extensions;

    use super::*;
    use crate::app_state::AccessTokenRecord;
    use crate::gyazo_api::GyazoUserProfile;
    use crate::runtime_config::RuntimeConfig;

    fn dummy_session(token: &str) -> AuthorizedSession {
        AuthorizedSession {
            record: AccessTokenRecord {
                backend_access_token: token.to_string(),
                gyazo_user: GyazoUserProfile {
                    email: String::new(),
                    name: String::new(),
                    profile_image: String::new(),
                    uid: String::new(),
                },
            },
        }
    }

    fn test_app_state() -> AppState {
        AppState::new_for_test(RuntimeConfig::for_test())
    }

    /// 0.6.2 で混入した回帰の再発防止テスト:
    ///
    /// HTTP 経由の tool 呼び出しで、middleware が `get_verified_session` 経由で
    /// 認証済みと判断していても、rmcp の StreamableHttpService は `Parts` 以外の
    /// extension を tool handler に転送しないため、Authorization ヘッダ無しで
    /// 来たリクエストは tool handler 側でセッションを取り戻せずエラーになっていた。
    ///
    /// ここでは「extensions に AuthorizedSession も Parts も無く、fallback も
    /// 無いが、`verified_session_cache` に検証済みセッションがある」状態を作り、
    /// `resolve_authorized_session` がそのキャッシュをフォールバックとして
    /// 返すことを保証する。
    #[tokio::test]
    async fn resolve_authorized_session_falls_back_to_verified_session_cache() {
        let app_state = test_app_state();

        // キャッシュに検証済みセッションを先行挿入 (Gyazo API には問い合わせない)
        let expected = dummy_session("cached-token");
        {
            let mut cache = app_state
                .verified_session_cache()
                .write()
                .expect("cache lock poisoned");
            *cache = Some((Instant::now(), Some(expected.clone())));
        }

        let extensions = Extensions::new();
        let result = resolve_authorized_session(&app_state, &extensions, None).await;

        let session = result.expect("cache fallback が働かずエラーになった");
        assert_eq!(
            session.record.backend_access_token,
            expected.record.backend_access_token,
        );
    }

    /// `AuthorizedSession` が extensions に直接注入されていれば、
    /// Gyazo API のキャッシュを覗くことなく即座にそれを返すこと。
    #[tokio::test]
    async fn resolve_authorized_session_prefers_injected_session() {
        let app_state = test_app_state();
        let injected = dummy_session("injected-token");

        let mut extensions = Extensions::new();
        extensions.insert(injected.clone());

        let result = resolve_authorized_session(&app_state, &extensions, None).await;

        let session = result.expect("injected session を返すべき");
        assert_eq!(session.record.backend_access_token, "injected-token");
    }

    /// stdio transport のケース: extensions に `Parts` も `AuthorizedSession`
    /// も無く、代わりに fallback session を持っている。フォールバックが
    /// そのまま返ること。
    #[tokio::test]
    async fn resolve_authorized_session_uses_fallback_when_no_extensions() {
        let app_state = test_app_state();
        let fallback = dummy_session("fallback-token");

        let extensions = Extensions::new();
        let result = resolve_authorized_session(&app_state, &extensions, Some(&fallback)).await;

        let session = result.expect("fallback を返すべき");
        assert_eq!(session.record.backend_access_token, "fallback-token");
    }

    /// どのソースからも取得できない場合は明確にエラーを返すこと
    /// (「request context に authorized session が含まれていません」)。
    #[tokio::test]
    async fn resolve_authorized_session_errors_when_nothing_available() {
        let app_state = test_app_state();

        let extensions = Extensions::new();
        let result = resolve_authorized_session(&app_state, &extensions, None).await;

        let error = result.expect_err("取得できないときはエラーを返すべき");
        assert!(
            error
                .message
                .contains("authorized session が含まれていません"),
            "想定外のエラーメッセージ: {}",
            error.message
        );
    }
}
