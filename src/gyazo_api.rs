use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};

const USERS_ME_URL: &str = "https://api.gyazo.com/api/users/me";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(crate) struct GyazoUserProfile {
    pub(crate) email: String,
    pub(crate) name: String,
    pub(crate) profile_image: String,
    pub(crate) uid: String,
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
    let status = response.status();
    let body = response
        .text()
        .await
        .context("failed to read Gyazo users/me response body")?;

    if !status.is_success() {
        bail!("Gyazo users/me failed with status {status}: {body}");
    }

    let parsed: UsersMeResponse =
        serde_json::from_str(&body).context("failed to parse Gyazo users/me response")?;

    Ok(parsed.user)
}
