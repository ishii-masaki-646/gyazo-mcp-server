use anyhow::Result;
use rmcp::{
    ErrorData as McpError,
    handler::server::wrapper::Parameters,
    model::{CallToolResult, Content},
    service::RequestContext,
};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::json;

use crate::{
    gyazo_api::{
        GyazoUploadImageRequest, delete_image, fetch_authenticated_user, fetch_image_as_base64,
        get_image, get_latest_image, get_oembed, list_images, search_images, upload_image,
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
    /// 組み立て済み `img_tag_html` の `<img>` タグに使う alt 属性。省略時は
    /// `"Gyazo image"`。Gyazo の oEmbed エンドポイントは title 等のリッチ
    /// メタデータを返さないため、呼び出し側で意味のある alt を指定したい
    /// 場合に使う。
    alt: Option<String>,
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
    async fn gyazo_whoami(
        &self,
        request_context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let session = self
            .authorized_session_for_request(&request_context)
            .await?;
        let user = fetch_authenticated_user(&session.record.backend_access_token)
            .await
            .map_err(internal_error)?;

        json_result(json!({
            "uid": user.uid,
            "name": user.name,
            "email": user.email,
            "profile_image": user.profile_image,
        }))
    }

    #[rmcp::tool(
        description = "現在の Gyazo ユーザーがアップロードしたキャプチャを全文検索します。各画像の戻り値に含まれる resource_uri (gyazo-mcp:///{image_id}) を MCP read_resource に渡すと、画像本体のバイナリを取得できます。"
    )]
    async fn gyazo_search(
        &self,
        request_context: RequestContext<rmcp::service::RoleServer>,
        Parameters(GyazoSearchArgs { query, page, per }): Parameters<GyazoSearchArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = self
            .authorized_session_for_request(&request_context)
            .await?;
        let images = search_images(&session.record.backend_access_token, &query, page, per)
            .await
            .map_err(internal_error)?;

        json_result(images)
    }

    #[rmcp::tool(
        description = "認証済みユーザーの Gyazo 画像一覧を取得します。各画像の戻り値に含まれる resource_uri (gyazo-mcp:///{image_id}) を MCP read_resource に渡すと、画像本体のバイナリを取得できます。"
    )]
    async fn gyazo_list_images(
        &self,
        request_context: RequestContext<rmcp::service::RoleServer>,
        Parameters(GyazoListImagesArgs { page, per_page }): Parameters<GyazoListImagesArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = self
            .authorized_session_for_request(&request_context)
            .await?;
        let images = list_images(&session.record.backend_access_token, page, per_page)
            .await
            .map_err(internal_error)?;

        json_result(images)
    }

    #[rmcp::tool(
        description = "画像 ID または Gyazo URL を指定して 1 件の画像を取得します。戻り値に含まれる resource_uri (gyazo-mcp:///{image_id}) を MCP read_resource に渡すと、画像本体のバイナリを取得できます。"
    )]
    async fn gyazo_get_image(
        &self,
        request_context: RequestContext<rmcp::service::RoleServer>,
        Parameters(args): Parameters<GyazoGetImageArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = self
            .authorized_session_for_request(&request_context)
            .await?;
        let image_ref = select_image_ref(args)?;
        let image = get_image(&session.record.backend_access_token, &image_ref)
            .await
            .map_err(internal_error)?;

        json_result(image)
    }

    #[rmcp::tool(description = "画像 ID または Gyazo URL を指定して 1 件の画像を削除します")]
    async fn gyazo_delete_image(
        &self,
        request_context: RequestContext<rmcp::service::RoleServer>,
        Parameters(args): Parameters<GyazoGetImageArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = self
            .authorized_session_for_request(&request_context)
            .await?;
        let image_ref = select_image_ref(args)?;
        let deleted = delete_image(&session.record.backend_access_token, &image_ref)
            .await
            .map_err(internal_error)?;

        json_result(deleted)
    }

    #[rmcp::tool(
        description = "最新の Gyazo 画像を画像本体とメタデータ付きで取得します。戻り値のメタデータには resource_uri (gyazo-mcp:///{image_id}) も含まれ、後から MCP read_resource に渡すことで画像本体のバイナリを再取得できます。"
    )]
    async fn gyazo_get_latest_image(
        &self,
        request_context: RequestContext<rmcp::service::RoleServer>,
    ) -> Result<CallToolResult, McpError> {
        let session = self
            .authorized_session_for_request(&request_context)
            .await?;
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
        request_context: RequestContext<rmcp::service::RoleServer>,
        Parameters(args): Parameters<GyazoUploadImageArgs>,
    ) -> Result<CallToolResult, McpError> {
        let session = self
            .authorized_session_for_request(&request_context)
            .await?;
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

    #[rmcp::tool(
        description = "Gyazo 画像ページ URL の oEmbed メタデータを取得します。戻り値の oembed_discovery_link は oEmbed spec 第 4 章で定義された discovery 用の <link rel=\"alternate\" type=\"application/json+oembed\" ...> タグで、HTML ページの <head> に置くと oEmbed 対応クライアントが埋め込み情報を発見できます。img_tag_html は spec 外の便利機能で、url / width / height と alt から組み立て済みの <img> タグです (markdown / HTML にそのまま貼って画像埋め込みに使えます)。oEmbed エンドポイントは title 等のリッチメタデータを返さないため、画像のタイトルや説明文等が必要な場合は gyazo_get_image を併用してください。"
    )]
    async fn gyazo_get_oembed_metadata(
        &self,
        Parameters(GyazoOEmbedArgs { image_url, alt }): Parameters<GyazoOEmbedArgs>,
    ) -> Result<CallToolResult, McpError> {
        let oembed = get_oembed(&image_url).await.map_err(internal_error)?;
        let img_tag_html = build_oembed_img_tag(
            &oembed.url,
            oembed.width,
            oembed.height,
            alt.as_deref().unwrap_or(DEFAULT_OEMBED_IMG_ALT),
        );
        let oembed_discovery_link = build_oembed_discovery_link(&image_url);

        json_result(json!({
            "version": oembed.version,
            "type": oembed.embed_type,
            "provider_name": oembed.provider_name,
            "provider_url": oembed.provider_url,
            "url": oembed.url,
            "width": oembed.width,
            "height": oembed.height,
            "oembed_discovery_link": oembed_discovery_link,
            "img_tag_html": img_tag_html,
        }))
    }
}

/// `gyazo_get_oembed_metadata` の `alt` 引数が省略されたときに
/// `img_tag_html` の `alt` 属性に使うデフォルト値。
const DEFAULT_OEMBED_IMG_ALT: &str = "Gyazo image";

/// oEmbed spec 第 4 章で定義された discovery link を組み立てる。
/// HTML ページの `<head>` に置くと、oEmbed 対応クライアント (クローラ等) が
/// この URL の埋め込み情報を発見できる。
fn build_oembed_discovery_link(image_page_url: &str) -> String {
    let encoded = percent_encode_query_value(image_page_url);
    format!(
        "<link rel=\"alternate\" type=\"application/json+oembed\" href=\"https://api.gyazo.com/api/oembed?url={encoded}\" title=\"Image shared with Gyazo\" />"
    )
}

/// `<img src="..." width="..." height="..." alt="..." />` を組み立てる。
/// `src` と `alt` は HTML 属性向けに最小限のエスケープ (`&`, `<`, `>`, `"`)
/// を行う。`width` / `height` は数値なのでエスケープ不要。
fn build_oembed_img_tag(src: &str, width: u64, height: u64, alt: &str) -> String {
    format!(
        "<img src=\"{src}\" width=\"{width}\" height=\"{height}\" alt=\"{alt}\" />",
        src = escape_html_attribute(src),
        alt = escape_html_attribute(alt),
    )
}

/// URL クエリ値向けの最小 percent エンコード。RFC 3986 の unreserved
/// 文字 (`A-Za-z0-9-._~`) 以外をすべて `%XX` に変換する。`&` や `=` も
/// エスケープされるので、エンコード後の文字列は HTML 属性内にそのまま
/// 埋め込んでも安全 (HTML エスケープが必要な特殊文字を含まない)。
fn percent_encode_query_value(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());
    for byte in value.bytes() {
        let unreserved = byte.is_ascii_alphanumeric()
            || byte == b'-'
            || byte == b'.'
            || byte == b'_'
            || byte == b'~';
        if unreserved {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }
    encoded
}

fn escape_html_attribute(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            _ => escaped.push(ch),
        }
    }
    escaped
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

#[cfg(test)]
mod tests {
    use super::{
        DEFAULT_OEMBED_IMG_ALT, build_oembed_discovery_link, build_oembed_img_tag,
        escape_html_attribute, percent_encode_query_value,
    };

    #[test]
    fn build_oembed_img_tag_uses_url_dimensions_and_alt() {
        let html = build_oembed_img_tag("https://i.gyazo.com/abc123.png", 640, 480, "screenshot");
        assert_eq!(
            html,
            "<img src=\"https://i.gyazo.com/abc123.png\" width=\"640\" height=\"480\" alt=\"screenshot\" />"
        );
    }

    #[test]
    fn build_oembed_img_tag_escapes_dangerous_chars_in_alt() {
        // alt にユーザー入力の "/<>/& が来ても属性脱出されないことを保証する。
        let html = build_oembed_img_tag("https://i.gyazo.com/abc.png", 10, 20, r#"a"b<c>d&e"#);
        assert!(html.contains("alt=\"a&quot;b&lt;c&gt;d&amp;e\""), "{html}");
        // 属性を閉じる素の `"` が alt 内に残っていないこと
        assert!(!html.contains("alt=\"a\""), "{html}");
    }

    #[test]
    fn build_oembed_img_tag_escapes_dangerous_chars_in_src() {
        // src は通常 Gyazo の URL なのでエスケープ不要だが、念のため & を含む
        // クエリ付き URL でも属性脱出しないことを保証する。
        let html = build_oembed_img_tag("https://i.gyazo.com/abc.png?a=1&b=2", 10, 20, "x");
        assert!(
            html.contains("src=\"https://i.gyazo.com/abc.png?a=1&amp;b=2\""),
            "{html}"
        );
    }

    #[test]
    fn escape_html_attribute_handles_all_special_chars() {
        assert_eq!(escape_html_attribute(r#"&<>"'"#), "&amp;&lt;&gt;&quot;'");
    }

    #[test]
    fn percent_encode_query_value_encodes_non_unreserved() {
        // Gyazo の典型的な画像ページ URL を percent-encode した結果を保証する。
        let encoded = percent_encode_query_value("https://gyazo.com/abc123");
        assert_eq!(encoded, "https%3A%2F%2Fgyazo.com%2Fabc123");
    }

    #[test]
    fn percent_encode_query_value_keeps_unreserved_only() {
        // 英数 + `-._~` だけがそのまま残ることを保証する。
        let encoded = percent_encode_query_value("AZaz09-._~");
        assert_eq!(encoded, "AZaz09-._~");
    }

    #[test]
    fn default_oembed_img_alt_is_gyazo_image() {
        // 回帰テスト: alt 引数が省略されたときに img_tag_html の alt に
        // 入るデフォルト値が "Gyazo image" であることを保証する。
        // ツール本体 (gyazo_get_oembed_metadata) が DEFAULT_OEMBED_IMG_ALT を
        // 参照しているので、この定数を変えるとデフォルト alt も変わる。
        assert_eq!(DEFAULT_OEMBED_IMG_ALT, "Gyazo image");

        // デフォルト値で組み立てた img タグが期待どおりになることも確認する。
        let html = build_oembed_img_tag(
            "https://i.gyazo.com/abc.png",
            10,
            20,
            DEFAULT_OEMBED_IMG_ALT,
        );
        assert!(
            html.contains("alt=\"Gyazo image\""),
            "デフォルト alt が img タグに反映されていません: {html}"
        );
    }

    #[test]
    fn build_oembed_discovery_link_matches_oembed_spec_form() {
        // oEmbed spec 第 4 章で定義された discovery link 形式
        // (`<link rel="alternate" type="application/json+oembed" href="..." />`)
        // を生成していることを保証する。Gyazo docs の例とも整合する。
        let link = build_oembed_discovery_link("https://gyazo.com/abc123");
        assert_eq!(
            link,
            "<link rel=\"alternate\" type=\"application/json+oembed\" \
             href=\"https://api.gyazo.com/api/oembed?url=https%3A%2F%2Fgyazo.com%2Fabc123\" \
             title=\"Image shared with Gyazo\" />"
        );
    }

    #[test]
    fn build_oembed_discovery_link_percent_encodes_url_query_param() {
        // image_page_url に `&` 等が含まれていても、href の query 値として
        // 安全に埋め込まれること (& が %26 に変換され、HTML 属性脱出も
        // 起こさないこと) を保証する。
        let link = build_oembed_discovery_link("https://gyazo.com/abc?x=1&y=2");
        assert!(
            link.contains("href=\"https://api.gyazo.com/api/oembed?url=https%3A%2F%2Fgyazo.com%2Fabc%3Fx%3D1%26y%3D2\""),
            "{link}"
        );
        // percent-encode 後は HTML 特殊文字が残らないので、`&amp;` への
        // HTML エスケープは不要 (= 出力に `&amp;` も生の `&` も無い)
        assert!(!link.contains("&amp;"), "{link}");
        assert!(!link.contains("=1&y"), "{link}");
    }
}
