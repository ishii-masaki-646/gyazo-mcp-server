use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use reqwest::{Url, multipart};
use serde::{Deserialize, Serialize};

const USERS_ME_URL: &str = "https://api.gyazo.com/api/users/me";
const LIST_IMAGES_URL: &str = "https://api.gyazo.com/api/images";
const GET_IMAGE_URL_PREFIX: &str = "https://api.gyazo.com/api/images/";
const UPLOAD_IMAGE_URL: &str = "https://upload.gyazo.com/api/upload";
const OEMBED_URL: &str = "https://api.gyazo.com/api/oembed";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoUserProfile {
    pub(crate) email: String,
    pub(crate) name: String,
    pub(crate) profile_image: String,
    pub(crate) uid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoImageMetadata {
    pub(crate) app: Option<String>,
    pub(crate) title: Option<String>,
    pub(crate) url: Option<String>,
    pub(crate) desc: String,
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
    pub(crate) metadata: GyazoImageMetadata,
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
    #[serde(rename = "type")]
    pub(crate) image_type: String,
    pub(crate) created_at: String,
    pub(crate) metadata: GyazoImageMetadata,
    pub(crate) ocr: Option<GyazoImageOcr>,
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
        .context("failed to call Gyazo users/me endpoint")?;
    parse_json_response::<UsersMeResponse>(response, "Gyazo users/me").await.map(|parsed| parsed.user)
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
        .context("failed to call Gyazo images list endpoint")?;
    let headers = response.headers().clone();
    let images = parse_json_response::<Vec<GyazoImageSummary>>(response, "Gyazo images list").await?;

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
        .context("failed to call Gyazo image detail endpoint")?;

    parse_json_response::<GyazoImageDetail>(response, "Gyazo image detail").await
}

pub(crate) async fn get_latest_image(access_token: &str) -> Result<GyazoImageSummary> {
    let listed = list_images(access_token, Some(1), Some(1)).await?;
    listed
        .images
        .into_iter()
        .next()
        .ok_or_else(|| anyhow!("Gyazo に画像がまだないよ"))
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
                .context("failed to set upload image mime type")?,
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
        .context("failed to call Gyazo upload endpoint")?;

    parse_json_response::<GyazoUploadImageResult>(response, "Gyazo upload").await
}

pub(crate) async fn get_oembed(image_url: &str) -> Result<GyazoOEmbedResponse> {
    let response = reqwest::Client::new()
        .get(OEMBED_URL)
        .query(&[("url", image_url)])
        .send()
        .await
        .context("failed to call Gyazo oEmbed endpoint")?;

    parse_json_response::<GyazoOEmbedResponse>(response, "Gyazo oEmbed").await
}

pub(crate) async fn fetch_image_as_base64(image_url: &str) -> Result<GyazoImageBinary> {
    let response = reqwest::Client::new()
        .get(image_url)
        .send()
        .await
        .context("failed to fetch Gyazo image bytes")?;
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
        .context("failed to read Gyazo image bytes")?;

    if !status.is_success() {
        bail!("Gyazo image fetch failed with status {status}");
    }

    Ok(GyazoImageBinary {
        data: STANDARD.encode(bytes),
        mime_type,
    })
}

async fn parse_json_response<T>(response: reqwest::Response, label: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let status = response.status();
    let body = response
        .text()
        .await
        .with_context(|| format!("failed to read {label} response body"))?;

    if !status.is_success() {
        bail!("{label} failed with status {status}: {body}");
    }

    serde_json::from_str(&body).with_context(|| format!("failed to parse {label} response"))
}

fn normalize_image_id(image_ref: &str) -> Result<String> {
    let trimmed = image_ref.trim();
    if trimmed.is_empty() {
        bail!("image_id か image_url を空にはできないよ");
    }

    if !trimmed.contains("://") {
        return Ok(trimmed.to_string());
    }

    let url = Url::parse(trimmed).context("image_url の形式が正しくないよ")?;
    match url.host_str() {
        Some("gyazo.com") | Some("www.gyazo.com") => url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            .map(|segment| segment.to_string())
            .ok_or_else(|| anyhow!("Gyazo ページ URL から image_id を取り出せなかったよ")),
        Some("i.gyazo.com") | Some("thumb.gyazo.com") => url
            .path_segments()
            .and_then(|mut segments| segments.rfind(|segment| !segment.is_empty()))
            .and_then(|filename| filename.split('.').next())
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.to_string())
            .ok_or_else(|| anyhow!("Gyazo 画像 URL から image_id を取り出せなかったよ")),
        _ => bail!("Gyazo の URL だけを受け付けるよ"),
    }
}

fn decode_image_data(image_data: &str) -> Result<Vec<u8>> {
    let payload = image_data
        .split_once("base64,")
        .map(|(_, encoded)| encoded)
        .unwrap_or(image_data)
        .trim();

    if payload.is_empty() {
        bail!("imageData が空だよ");
    }

    STANDARD
        .decode(payload)
        .context("imageData を base64 として読めなかったよ")
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
    use super::{decode_image_data, guess_mime_type_from_url, normalize_image_id};

    #[test]
    fn normalize_image_id_accepts_raw_id() {
        let actual = normalize_image_id("8980c52421e452ac3355ca3e5cfe7a0c").unwrap();
        assert_eq!(actual, "8980c52421e452ac3355ca3e5cfe7a0c");
    }

    #[test]
    fn normalize_image_id_accepts_gyazo_page_url() {
        let actual = normalize_image_id("https://gyazo.com/8980c52421e452ac3355ca3e5cfe7a0c").unwrap();
        assert_eq!(actual, "8980c52421e452ac3355ca3e5cfe7a0c");
    }

    #[test]
    fn normalize_image_id_accepts_gyazo_image_url() {
        let actual = normalize_image_id("https://i.gyazo.com/8980c52421e452ac3355ca3e5cfe7a0c.png").unwrap();
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
}
