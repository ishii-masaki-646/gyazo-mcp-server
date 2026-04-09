use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{Url, multipart};
use serde::{Deserialize, Serialize};

const USERS_ME_URL: &str = "https://api.gyazo.com/api/users/me";
const LIST_IMAGES_URL: &str = "https://api.gyazo.com/api/images";
const SEARCH_IMAGES_URL: &str = "https://api.gyazo.com/api/search";
const GET_IMAGE_URL_PREFIX: &str = "https://api.gyazo.com/api/images/";
const UPLOAD_IMAGE_URL: &str = "https://upload.gyazo.com/api/upload";
const OEMBED_URL: &str = "https://api.gyazo.com/api/oembed";
const RESOURCE_URI_PREFIX: &str = "gyazo-mcp:///";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct GyazoUserProfile {
    pub(crate) email: String,
    pub(crate) name: String,
    pub(crate) profile_image: String,
    pub(crate) uid: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct GyazoImageMetadata {
    #[serde(default)]
    pub(crate) app: Option<String>,
    #[serde(default)]
    pub(crate) title: Option<String>,
    #[serde(default)]
    pub(crate) url: Option<String>,
    /// metadata が非公開もしくは存在しない場合や、明示的に null で
    /// 返るレスポンスがあるため Option 扱い。自分画像でも空文字で来る
    /// ことが多い。
    #[serde(default)]
    pub(crate) desc: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoImageSummary {
    pub(crate) image_id: String,
    pub(crate) permalink_url: String,
    pub(crate) thumb_url: String,
    pub(crate) url: String,
    #[serde(rename = "type")]
    pub(crate) image_type: String,
    pub(crate) created_at: String,
    /// metadata が非公開もしくは存在しない画像では、Gyazo API がこの
    /// オブジェクトを丸ごと省略してくることがあるため Option 扱い。
    #[serde(default)]
    pub(crate) metadata: Option<GyazoImageMetadata>,
    /// MCP resource URI (`gyazo-mcp:///{image_id}`)。Gyazo API 側のレスポンスには
    /// 含まれないため、パース後に `populate_resource_uri()` で埋める。
    #[serde(default)]
    pub(crate) resource_uri: String,
}

impl GyazoImageSummary {
    fn populate_resource_uri(&mut self) {
        self.resource_uri = create_image_resource_uri(&self.image_id);
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoImageOcr {
    pub(crate) locale: String,
    pub(crate) description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoImageDetail {
    pub(crate) image_id: String,
    pub(crate) permalink_url: Option<String>,
    pub(crate) thumb_url: Option<String>,
    pub(crate) url: Option<String>,
    #[serde(rename = "type")]
    pub(crate) image_type: String,
    pub(crate) created_at: String,
    /// metadata が非公開 (`metadata_is_public: false` でアップロードされた
    /// 画像、メタデータを含まない画像、他人がアップロードした
    /// `access_policy: anyone` の画像など) の場合は、Gyazo API がこの
    /// オブジェクトを丸ごと省略してくることがあるため Option 扱い。
    #[serde(default)]
    pub(crate) metadata: Option<GyazoImageMetadata>,
    pub(crate) ocr: Option<GyazoImageOcr>,
    /// MCP resource URI (`gyazo-mcp:///{image_id}`)。Gyazo API 側のレスポンスには
    /// 含まれないため、パース後に `populate_resource_uri()` で埋める。
    #[serde(default)]
    pub(crate) resource_uri: String,
}

impl GyazoImageDetail {
    fn populate_resource_uri(&mut self) {
        self.resource_uri = create_image_resource_uri(&self.image_id);
    }
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct GyazoImageListResult {
    pub(crate) total_count: Option<u64>,
    pub(crate) current_page: Option<u64>,
    pub(crate) per_page: Option<u64>,
    pub(crate) user_type: Option<String>,
    pub(crate) images: Vec<GyazoImageSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoUploadImageResult {
    pub(crate) image_id: String,
    pub(crate) permalink_url: String,
    pub(crate) thumb_url: String,
    pub(crate) url: String,
    #[serde(rename = "type")]
    pub(crate) image_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoDeleteImageResult {
    pub(crate) image_id: String,
    #[serde(rename = "type")]
    pub(crate) image_type: String,
}

#[derive(Debug, Clone)]
pub(crate) struct GyazoUploadImageRequest {
    pub(crate) image_data: String,
    pub(crate) access_policy: Option<String>,
    pub(crate) metadata_is_public: Option<bool>,
    pub(crate) referer_url: Option<String>,
    pub(crate) app: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) description: Option<String>,
    pub(crate) created_at: Option<f64>,
    pub(crate) collection_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoOEmbedResponse {
    pub(crate) version: String,
    #[serde(rename = "type")]
    pub(crate) embed_type: String,
    pub(crate) provider_name: String,
    pub(crate) provider_url: String,
    pub(crate) url: String,
    pub(crate) width: u64,
    pub(crate) height: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoImageBinary {
    pub(crate) data: String,
    pub(crate) mime_type: String,
}

#[derive(Debug, Deserialize)]
struct UsersMeResponse {
    user: GyazoUserProfile,
}

pub(crate) async fn fetch_authenticated_user(access_token: &str) -> Result<GyazoUserProfile> {
    let response = reqwest::Client::new()
        .get(USERS_ME_URL)
        .bearer_auth(access_token)
        .send()
        .await
        .context("Gyazo users/me endpoint の呼び出しに失敗しました")?;
    parse_json_response::<UsersMeResponse>(response, "Gyazo users/me")
        .await
        .map(|parsed| parsed.user)
}

pub(crate) async fn list_images(
    access_token: &str,
    page: Option<u32>,
    per_page: Option<u32>,
) -> Result<GyazoImageListResult> {
    let response = reqwest::Client::new()
        .get(LIST_IMAGES_URL)
        .query(&[
            ("access_token", access_token.to_string()),
            ("page", page.unwrap_or(1).to_string()),
            ("per_page", per_page.unwrap_or(20).to_string()),
        ])
        .send()
        .await
        .context("Gyazo images list endpoint の呼び出しに失敗しました")?;
    let headers = response.headers().clone();
    let mut images =
        parse_json_response::<Vec<GyazoImageSummary>>(response, "Gyazo images list").await?;
    for image in &mut images {
        image.populate_resource_uri();
    }

    Ok(GyazoImageListResult {
        total_count: header_u64(&headers, "X-Total-Count"),
        current_page: header_u64(&headers, "X-Current-Page"),
        per_page: header_u64(&headers, "X-Per-Page"),
        user_type: header_string(&headers, "X-User-Type"),
        images,
    })
}

pub(crate) async fn get_image(access_token: &str, image_ref: &str) -> Result<GyazoImageDetail> {
    let image_id = normalize_image_id(image_ref)?;
    let response = reqwest::Client::new()
        .get(format!("{GET_IMAGE_URL_PREFIX}{image_id}"))
        .query(&[("access_token", access_token)])
        .send()
        .await
        .context("Gyazo image detail endpoint の呼び出しに失敗しました")?;

    let mut detail =
        parse_json_response::<GyazoImageDetail>(response, "Gyazo image detail").await?;
    detail.populate_resource_uri();
    Ok(detail)
}

pub(crate) async fn delete_image(
    access_token: &str,
    image_ref: &str,
) -> Result<GyazoDeleteImageResult> {
    let image_id = normalize_image_id(image_ref)?;
    let response = reqwest::Client::new()
        .delete(format!("{GET_IMAGE_URL_PREFIX}{image_id}"))
        .query(&[("access_token", access_token)])
        .send()
        .await
        .context("Gyazo image delete endpoint の呼び出しに失敗しました")?;

    parse_json_response::<GyazoDeleteImageResult>(response, "Gyazo image delete").await
}

pub(crate) async fn get_latest_image(access_token: &str) -> Result<GyazoImageSummary> {
    let listed = list_images(access_token, Some(1), Some(1)).await?;
    listed
        .images
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Gyazo に画像がまだありません"))
}

pub(crate) async fn search_images(
    access_token: &str,
    query: &str,
    page: Option<u32>,
    per: Option<u32>,
) -> Result<Vec<GyazoImageDetail>> {
    let trimmed_query = query.trim();
    if trimmed_query.is_empty() {
        bail!("query は空にできません");
    }
    if trimmed_query.chars().count() > 200 {
        bail!("query は 200 文字以内で指定してください");
    }

    let per = per.unwrap_or(20);
    if !(1..=100).contains(&per) {
        bail!("per は 1 から 100 の範囲で指定してください");
    }

    let response = reqwest::Client::new()
        .get(SEARCH_IMAGES_URL)
        .query(&[
            ("access_token", access_token.to_string()),
            ("query", trimmed_query.to_string()),
            ("page", page.unwrap_or(1).to_string()),
            ("per", per.to_string()),
        ])
        .send()
        .await
        .context("Gyazo search endpoint の呼び出しに失敗しました")?;

    if response.status() == reqwest::StatusCode::PAYMENT_REQUIRED {
        let body = response
            .text()
            .await
            .context("Gyazo search の課金エラーレスポンス本文を読み取れませんでした")?;
        bail!(
            "Gyazo Search を利用するには Pro プランが必要です (status 402 Payment Required: {body})"
        );
    }

    let mut images = parse_json_response::<Vec<GyazoImageDetail>>(response, "Gyazo search").await?;
    for image in &mut images {
        image.populate_resource_uri();
    }
    Ok(images)
}

pub(crate) async fn upload_image(
    access_token: &str,
    request: GyazoUploadImageRequest,
) -> Result<GyazoUploadImageResult> {
    let image_bytes = decode_image_data(&request.image_data)?;
    let mut form = multipart::Form::new()
        .text("access_token", access_token.to_string())
        .part(
            "imagedata",
            multipart::Part::bytes(image_bytes)
                .file_name("upload.png")
                .mime_str("image/png")
                .context("アップロード画像の MIME type を設定できませんでした")?,
        );

    if let Some(access_policy) = request.access_policy {
        form = form.text("access_policy", access_policy);
    }
    if let Some(metadata_is_public) = request.metadata_is_public {
        form = form.text(
            "metadata_is_public",
            if metadata_is_public { "true" } else { "false" }.to_string(),
        );
    }
    if let Some(referer_url) = request.referer_url {
        form = form.text("referer_url", referer_url);
    }
    if let Some(app) = request.app {
        form = form.text("app", app);
    }
    if let Some(title) = request.title {
        form = form.text("title", title);
    }
    if let Some(description) = request.description {
        form = form.text("desc", description);
    }
    if let Some(created_at) = request.created_at {
        form = form.text("created_at", created_at.to_string());
    }
    if let Some(collection_id) = request.collection_id {
        form = form.text("collection_id", collection_id);
    }

    let response = reqwest::Client::new()
        .post(UPLOAD_IMAGE_URL)
        .multipart(form)
        .send()
        .await
        .context("Gyazo upload endpoint の呼び出しに失敗しました")?;

    parse_json_response::<GyazoUploadImageResult>(response, "Gyazo upload").await
}

pub(crate) async fn get_oembed(image_url: &str) -> Result<GyazoOEmbedResponse> {
    let response = reqwest::Client::new()
        .get(OEMBED_URL)
        .query(&[("url", image_url)])
        .send()
        .await
        .context("Gyazo oEmbed endpoint の呼び出しに失敗しました")?;

    parse_json_response::<GyazoOEmbedResponse>(response, "Gyazo oEmbed").await
}

pub(crate) async fn fetch_image_as_base64(image_url: &str) -> Result<GyazoImageBinary> {
    let response = reqwest::Client::new()
        .get(image_url)
        .send()
        .await
        .context("Gyazo 画像のバイト列を取得できませんでした")?;
    let status = response.status();
    let mime_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
        .unwrap_or_else(|| guess_mime_type_from_url(image_url));
    let bytes = response
        .bytes()
        .await
        .context("Gyazo 画像のバイト列を読み取れませんでした")?;

    if !status.is_success() {
        bail!("Gyazo 画像の取得に失敗しました (status {status})");
    }

    Ok(GyazoImageBinary {
        data: STANDARD.encode(bytes),
        mime_type,
    })
}

pub(crate) fn create_image_resource_uri(image_id: &str) -> String {
    format!("{RESOURCE_URI_PREFIX}{image_id}")
}

pub(crate) fn extract_image_id_from_resource_uri(uri: &str) -> Result<String> {
    uri.strip_prefix(RESOURCE_URI_PREFIX)
        .filter(|image_id| !image_id.trim().is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| anyhow!("resource URI は gyazo-mcp:///image_id の形式で指定してください"))
}

pub(crate) fn format_image_metadata_markdown(image: &GyazoImageDetail) -> String {
    let mut markdown = String::new();

    let Some(metadata) = image.metadata.as_ref() else {
        return markdown;
    };

    if let Some(title) = metadata.title.as_deref()
        && !title.is_empty()
    {
        markdown.push_str("### Title:\n");
        markdown.push_str(title);
        markdown.push_str("\n\n");
    }
    if let Some(desc) = metadata.desc.as_deref()
        && !desc.is_empty()
    {
        markdown.push_str("### Description:\n");
        markdown.push_str(desc);
        markdown.push_str("\n\n");
    }
    if let Some(app) = metadata.app.as_deref()
        && !app.is_empty()
    {
        markdown.push_str("### App:\n");
        markdown.push_str(app);
        markdown.push_str("\n\n");
    }
    if let Some(url) = metadata.url.as_deref()
        && !url.is_empty()
    {
        markdown.push_str("### URL:\n");
        markdown.push_str(url);
        markdown.push_str("\n\n");
    }
    if let Some(ocr) = image.ocr.as_ref() {
        if !ocr.description.is_empty() {
            markdown.push_str("### OCR:\n");
            markdown.push_str(&ocr.description);
            markdown.push_str("\n\n");
        }
        if !ocr.locale.is_empty() {
            markdown.push_str("### OCR Locale:\n");
            markdown.push_str(&ocr.locale);
            markdown.push_str("\n\n");
        }
    }

    markdown
}

async fn parse_json_response<T>(response: reqwest::Response, label: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("{label} のレスポンス本文を読み取れませんでした"))?;

    if !status.is_success() {
        bail!("{label} が失敗しました (status {status}: {body})");
    }

    serde_json::from_str(&body)
        .with_context(|| format!("{label} のレスポンスを解析できませんでした"))
}

fn normalize_image_id(image_ref: &str) -> Result<String> {
    let trimmed = image_ref.trim();
    if trimmed.is_empty() {
        bail!("image_id または image_url を指定してください");
    }

    if !trimmed.contains("://") {
        return Ok(trimmed.to_string());
    }

    let url = Url::parse(trimmed).context("image_url の形式が正しくありません")?;
    match url.host_str() {
        Some("gyazo.com") | Some("www.gyazo.com") => url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            .map(|segment| segment.to_string())
            .ok_or_else(|| anyhow!("Gyazo ページ URL から image_id を取得できませんでした")),
        Some("i.gyazo.com") | Some("thumb.gyazo.com") => url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            .and_then(|filename| filename.split('.').next())
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
            .ok_or_else(|| anyhow!("Gyazo 画像 URL から image_id を取得できませんでした")),
        _ => bail!("Gyazo の URL のみ指定できます"),
    }
}

fn decode_image_data(image_data: &str) -> Result<Vec<u8>> {
    let payload = image_data
        .split_once("base64,")
        .map(|(_, encoded)| encoded)
        .unwrap_or(image_data)
        .trim();

    if payload.is_empty() {
        bail!("imageData が空です");
    }

    STANDARD
        .decode(payload)
        .context("imageData を base64 として読み取れませんでした")
}

fn header_u64(headers: &reqwest::header::HeaderMap, name: &str) -> Option<u64> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
}

fn header_string(headers: &reqwest::header::HeaderMap, name: &str) -> Option<String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(ToOwned::to_owned)
}

fn guess_mime_type_from_url(image_url: &str) -> String {
    let lower = image_url.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png".to_string()
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg".to_string()
    } else if lower.ends_with(".gif") {
        "image/gif".to_string()
    } else if lower.ends_with(".webp") {
        "image/webp".to_string()
    } else {
        "application/octet-stream".to_string()
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        GyazoImageDetail, GyazoImageSummary, create_image_resource_uri, decode_image_data,
        extract_image_id_from_resource_uri, guess_mime_type_from_url, normalize_image_id,
    };

    #[test]
    fn normalize_image_id_accepts_raw_id() {
        let actual = normalize_image_id("8980c52421e452ac3355ca3e5cfe7a0c").unwrap();
        assert_eq!(actual, "8980c52421e452ac3355ca3e5cfe7a0c");
    }

    #[test]
    fn normalize_image_id_accepts_gyazo_page_url() {
        let actual =
            normalize_image_id("https://gyazo.com/8980c52421e452ac3355ca3e5cfe7a0c").unwrap();
        assert_eq!(actual, "8980c52421e452ac3355ca3e5cfe7a0c");
    }

    #[test]
    fn normalize_image_id_accepts_gyazo_image_url() {
        let actual =
            normalize_image_id("https://i.gyazo.com/8980c52421e452ac3355ca3e5cfe7a0c.png").unwrap();
        assert_eq!(actual, "8980c52421e452ac3355ca3e5cfe7a0c");
    }

    #[test]
    fn normalize_image_id_rejects_non_gyazo_url() {
        let error = normalize_image_id("https://example.com/image.png").unwrap_err();
        assert!(error.to_string().contains("Gyazo の URL"));
    }

    #[test]
    fn decode_image_data_accepts_data_url_prefix() {
        let actual = decode_image_data("data:image/png;base64,SGVsbG8=").unwrap();
        assert_eq!(actual, b"Hello");
    }

    #[test]
    fn guess_mime_type_from_url_prefers_extension() {
        let actual = guess_mime_type_from_url("https://i.gyazo.com/example.JPG");
        assert_eq!(actual, "image/jpeg");
    }

    #[test]
    fn create_and_extract_resource_uri_roundtrip() {
        let uri = create_image_resource_uri("abc123");
        assert_eq!(uri, "gyazo-mcp:///abc123");
        let image_id = extract_image_id_from_resource_uri(&uri).unwrap();
        assert_eq!(image_id, "abc123");
    }

    /// Gyazo API のレスポンスサンプル (resource_uri を含まない) をデシリアライズし、
    /// `populate_resource_uri()` 後にシリアライズすると `gyazo-mcp:///{image_id}`
    /// が乗ることを保証する回帰テスト。`#[serde(default)]` の欠落や、
    /// `populate_resource_uri()` の呼び忘れに気付けるようにする。
    #[test]
    fn image_summary_populates_resource_uri() {
        let api_response = json!({
            "image_id": "abc123",
            "permalink_url": "https://gyazo.com/abc123",
            "thumb_url": "https://i.gyazo.com/thumb/abc123.png",
            "url": "https://i.gyazo.com/abc123.png",
            "type": "png",
            "created_at": "2026-04-09T00:00:00+0900",
            "metadata": {
                "app": null,
                "title": null,
                "url": null,
                "desc": ""
            }
        });

        let mut summary: GyazoImageSummary =
            serde_json::from_value(api_response).expect("API レスポンスのパースに失敗");
        assert_eq!(
            summary.resource_uri, "",
            "API レスポンスに resource_uri が無いので初期状態は空のはず"
        );

        summary.populate_resource_uri();
        assert_eq!(summary.resource_uri, "gyazo-mcp:///abc123");

        let serialized = serde_json::to_value(&summary).unwrap();
        assert_eq!(
            serialized.get("resource_uri").and_then(|v| v.as_str()),
            Some("gyazo-mcp:///abc123"),
            "シリアライズ結果に resource_uri が含まれていません"
        );
    }

    #[test]
    fn image_detail_populates_resource_uri() {
        let api_response = json!({
            "image_id": "def456",
            "permalink_url": "https://gyazo.com/def456",
            "thumb_url": "https://i.gyazo.com/thumb/def456.png",
            "url": "https://i.gyazo.com/def456.png",
            "type": "png",
            "created_at": "2026-04-09T00:00:00+0900",
            "metadata": {
                "app": null,
                "title": null,
                "url": null,
                "desc": ""
            },
            "ocr": null
        });

        let mut detail: GyazoImageDetail =
            serde_json::from_value(api_response).expect("API レスポンスのパースに失敗");
        assert_eq!(detail.resource_uri, "");

        detail.populate_resource_uri();
        assert_eq!(detail.resource_uri, "gyazo-mcp:///def456");

        let serialized = serde_json::to_value(&detail).unwrap();
        assert_eq!(
            serialized.get("resource_uri").and_then(|v| v.as_str()),
            Some("gyazo-mcp:///def456"),
        );
    }

    /// `GET /api/images/:id` のレスポンスは、metadata が非公開
    /// (`metadata_is_public: false` でアップロードされた画像、メタデータを
    /// 含まない画像、他人がアップロードした `access_policy: anyone` の
    /// 画像など) の場合、`metadata` フィールド自体を丸ごと省略してくる
    /// ことがある。`metadata: GyazoImageMetadata` (Non-Optional) のままだと
    /// "missing field `metadata`" でパース失敗するため、Option 化した
    /// うえで再発防止のテストとして固定する。
    #[test]
    fn image_detail_parses_response_without_metadata_field() {
        // 実際の Gyazo API レスポンス例 (metadata が含まれていないケース)
        let api_response = json!({
            "image_id": "65cb43fa2b8de2c8d9538cec36c8249d",
            "type": "png",
            "created_at": "2026-04-07T07:19:34.183Z",
            "permalink_url": "https://gyazo.com/65cb43fa2b8de2c8d9538cec36c8249d",
            "thumb_url": "https://thumb.gyazo.com/thumb/200/example.jpg",
            "url": "https://i.gyazo.com/65cb43fa2b8de2c8d9538cec36c8249d.png",
            "access_policy": "anyone"
            // metadata と ocr は丸ごと存在しない
        });

        let detail: GyazoImageDetail = serde_json::from_value(api_response)
            .expect("metadata 欠落のレスポンスがパースできませんでした");
        assert!(detail.metadata.is_none(), "metadata は None として扱うべき");
        assert!(detail.ocr.is_none());
        assert_eq!(detail.image_id, "65cb43fa2b8de2c8d9538cec36c8249d");
    }

    /// `metadata` オブジェクトはあるが、内部の各フィールドが部分的に
    /// 欠落 / null のケースもパースできることを保証する。
    #[test]
    fn image_detail_parses_response_with_partial_metadata_fields() {
        // app だけある、他は無いケース
        let api_response = json!({
            "image_id": "abc",
            "type": "png",
            "created_at": "2026-04-09T00:00:00Z",
            "permalink_url": "https://gyazo.com/abc",
            "thumb_url": "https://i.gyazo.com/thumb/abc.png",
            "url": "https://i.gyazo.com/abc.png",
            "metadata": {
                "app": "Gyazo"
                // title / url / desc は欠落
            }
        });

        let detail: GyazoImageDetail = serde_json::from_value(api_response)
            .expect("部分的な metadata のレスポンスがパースできませんでした");
        let metadata = detail.metadata.expect("metadata は Some であるべき");
        assert_eq!(metadata.app.as_deref(), Some("Gyazo"));
        assert!(metadata.title.is_none());
        assert!(metadata.url.is_none());
        assert!(metadata.desc.is_none());
    }

    /// `metadata` の各フィールドが明示的に `null` で来ているケース
    /// (例: `desc` が null)。`desc: String` (Non-Optional) のままだと
    /// null でパース失敗するため、各フィールドを `Option<String>` 化
    /// したうえで再発防止のテストとして固定する。
    #[test]
    fn image_detail_parses_response_with_null_metadata_fields() {
        let api_response = json!({
            "image_id": "abc",
            "type": "png",
            "created_at": "2026-04-09T00:00:00Z",
            "permalink_url": "https://gyazo.com/abc",
            "thumb_url": "https://i.gyazo.com/thumb/abc.png",
            "url": "https://i.gyazo.com/abc.png",
            "metadata": {
                "app": null,
                "title": null,
                "url": null,
                "desc": null
            }
        });

        let detail: GyazoImageDetail = serde_json::from_value(api_response)
            .expect("null フィールド入りの metadata がパースできませんでした");
        let metadata = detail.metadata.expect("metadata は Some であるべき");
        assert!(metadata.app.is_none());
        assert!(metadata.title.is_none());
        assert!(metadata.url.is_none());
        assert!(metadata.desc.is_none());
    }
}
