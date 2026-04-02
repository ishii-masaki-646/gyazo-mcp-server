use anyhow::{Context, Result, anyhow, bail};
use axum::http::StatusCode;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    app_state::AppState,
    auth::token_store::StoredToken,
};

const AUTHORIZE_URL: &str = "https://gyazo.com/oauth/authorize";
const TOKEN_URL: &str = "https://gyazo.com/oauth/token";

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct OAuthCallbackQuery {
    pub(crate) code: Option<String>,
    pub(crate) error: Option<String>,
    pub(crate) error_description: Option<String>,
    pub(crate) state: Option<String>,
}

#[derive(Debug)]
pub(crate) struct OAuthCallbackFailure {
    message: String,
    status_code: StatusCode,
}

impl OAuthCallbackFailure {
    pub(crate) fn bad_request(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: StatusCode::BAD_REQUEST,
        }
    }

    pub(crate) fn internal(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    pub(crate) fn bad_gateway(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            status_code: StatusCode::BAD_GATEWAY,
        }
    }

    pub(crate) fn into_parts(self) -> (StatusCode, String) {
        (self.status_code, self.message)
    }
}

pub(crate) fn begin_login(app_state: &AppState) -> Result<String> {
    let state = Uuid::new_v4().to_string();

    app_state.set_pending_direct_login_state(state.clone())?;

    build_gyazo_authorize_url(app_state, &state)
}

pub(crate) async fn complete_login(
    app_state: &AppState,
    query: OAuthCallbackQuery,
) -> Result<String, OAuthCallbackFailure> {
    let OAuthCallbackQuery {
        code,
        error,
        error_description,
        state,
    } = query;

    if let Some(error) = error {
        let description = error_description.unwrap_or_default();
        let suffix = if description.is_empty() {
            String::new()
        } else {
            format!(": {description}")
        };
        return Err(OAuthCallbackFailure::bad_request(format!(
            "Gyazo OAuth がエラーを返したよ ({error}{suffix})"
        )));
    }

    let code = code
        .ok_or_else(|| OAuthCallbackFailure::bad_request("callback に code が含まれていないよ"))?;
    let returned_state = state
        .ok_or_else(|| OAuthCallbackFailure::bad_request("callback に state が含まれていないよ"))?;
    let pending_state = app_state
        .take_pending_direct_login_state()
        .map_err(|error| OAuthCallbackFailure::internal(error.to_string()))?
        .ok_or_else(|| {
            OAuthCallbackFailure::bad_request(
                "保留中の OAuth state が見つからないよ。/oauth/start からやり直してね",
            )
        })?;

    if returned_state != pending_state {
        return Err(OAuthCallbackFailure::bad_request(
            "OAuth state が一致しないよ。もう一度 login をやり直してね",
        ));
    }

    let token = exchange_code_for_token(app_state, &code)
        .await
        .map_err(|error| OAuthCallbackFailure::bad_gateway(error.to_string()))?;

    app_state
        .save_oauth_token(token)
        .map_err(|error| OAuthCallbackFailure::internal(error.to_string()))?;

    let token_path = app_state
        .auth_state_snapshot()
        .map_err(|error| OAuthCallbackFailure::internal(error.to_string()))?
        .token_file_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "(unavailable)".to_string());

    Ok(format!(
        "Gyazo OAuth login が完了したよ。token は {token_path} に保存したわ。"
    ))
}

pub(crate) fn build_gyazo_authorize_url(app_state: &AppState, state: &str) -> Result<String> {
    let credentials = app_state
        .auth_config()
        .oauth_credentials()
        .context("GYAZO_MCP_OAUTH_CLIENT_ID と GYAZO_MCP_OAUTH_CLIENT_SECRET を設定してね")?;

    Ok(build_authorize_url(
        &credentials.client_id,
        &app_state.runtime_config().oauth_callback_url(),
        state,
    ))
}

fn build_authorize_url(client_id: &str, redirect_uri: &str, state: &str) -> String {
    let query = [
        ("client_id", client_id),
        ("redirect_uri", redirect_uri),
        ("response_type", "code"),
        ("state", state),
    ]
    .into_iter()
    .map(|(key, value)| format!("{key}={}", percent_encode(value)))
    .collect::<Vec<_>>()
    .join("&");

    format!("{AUTHORIZE_URL}?{query}")
}

pub(crate) async fn exchange_code_for_token(app_state: &AppState, code: &str) -> Result<StoredToken> {
    let credentials = app_state
        .auth_config()
        .oauth_credentials()
        .context("GYAZO_MCP_OAUTH_CLIENT_ID と GYAZO_MCP_OAUTH_CLIENT_SECRET を設定してね")?;
    let redirect_uri = app_state.runtime_config().oauth_callback_url();
    let response = reqwest::Client::new()
        .post(TOKEN_URL)
        .form(&[
            ("client_id", credentials.client_id.as_str()),
            ("client_secret", credentials.client_secret.as_str()),
            ("code", code),
            ("grant_type", "authorization_code"),
            ("redirect_uri", redirect_uri.as_str()),
        ])
        .send()
        .await
        .context("failed to call Gyazo token endpoint")?;
    let status = response.status();
    let body = response
        .text()
        .await
        .context("failed to read Gyazo token endpoint response body")?;

    if !status.is_success() {
        bail!("Gyazo token exchange failed with status {status}: {body}");
    }

    let parsed: GyazoTokenResponse =
        serde_json::from_str(&body).context("failed to parse Gyazo token response")?;

    if parsed.access_token.trim().is_empty() {
        return Err(anyhow!("Gyazo token response did not include access_token"));
    }

    Ok(StoredToken {
        access_token: parsed.access_token,
    })
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len());

    for byte in value.bytes() {
        let is_unreserved =
            byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.' | b'~');

        if is_unreserved {
            encoded.push(byte as char);
        } else {
            encoded.push_str(&format!("%{:02X}", byte));
        }
    }

    encoded
}

#[derive(Debug, Deserialize)]
struct GyazoTokenResponse {
    access_token: String,
}

#[cfg(test)]
mod tests {
    use super::build_authorize_url;

    #[test]
    fn builds_authorize_url_with_required_parameters() {
        let url = build_authorize_url(
            "client-id",
            "http://127.0.0.1:18449/oauth/callback",
            "state-123",
        );

        assert!(url.starts_with("https://gyazo.com/oauth/authorize?"));
        assert!(url.contains("client_id=client-id"));
        assert!(
            url.contains("redirect_uri=http%3A%2F%2F127.0.0.1%3A18449%2Foauth%2Fcallback")
        );
        assert!(url.contains("response_type=code"));
        assert!(url.contains("state=state-123"));
    }
}
