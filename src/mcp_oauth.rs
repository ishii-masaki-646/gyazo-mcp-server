use std::sync::Arc;

use anyhow::{Result, anyhow, bail};
use axum::{
    Form, Json,
    body::Body,
    extract::{Query, State},
    http::{
        HeaderValue, Request, StatusCode,
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
        request::Parts,
    },
    middleware::Next,
    response::{IntoResponse, Redirect, Response},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{
    app_state::{
        AccessTokenRecord, AppState, AuthorizationCodeGrant, AuthorizedSession,
        PendingAuthorizationRequest, RegisteredClient,
    },
    auth::oauth::{
        OAuthCallbackFailure, OAuthCallbackQuery, build_gyazo_authorize_url,
        exchange_code_for_token,
    },
    gyazo_api::fetch_authenticated_user,
};

const REQUIRED_SCOPE: &str = "gyazo";

#[derive(Debug, Serialize)]
pub(crate) struct ProtectedResourceMetadata {
    resource: String,
    authorization_servers: Vec<String>,
    scopes_supported: Vec<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct AuthorizationServerMetadata {
    issuer: String,
    authorization_endpoint: String,
    token_endpoint: String,
    registration_endpoint: String,
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
    token_endpoint_auth_methods_supported: Vec<String>,
    code_challenge_methods_supported: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct AuthorizationRequestQuery {
    response_type: Option<String>,
    client_id: Option<String>,
    redirect_uri: Option<String>,
    state: Option<String>,
    code_challenge: Option<String>,
    code_challenge_method: Option<String>,
    scope: Option<String>,
    resource: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct TokenRequestForm {
    grant_type: Option<String>,
    code: Option<String>,
    redirect_uri: Option<String>,
    client_id: Option<String>,
    code_verifier: Option<String>,
    resource: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct TokenResponse {
    access_token: String,
    token_type: String,
    scope: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct DynamicClientRegistrationRequest {
    redirect_uris: Option<Vec<String>>,
    client_name: Option<String>,
    grant_types: Option<Vec<String>>,
    response_types: Option<Vec<String>>,
    token_endpoint_auth_method: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct DynamicClientRegistrationResponse {
    client_id: String,
    redirect_uris: Vec<String>,
    client_name: Option<String>,
    grant_types: Vec<String>,
    response_types: Vec<String>,
    token_endpoint_auth_method: String,
}

pub(crate) async fn require_mcp_bearer_token(
    State(app_state): State<Arc<AppState>>,
    mut request: Request<Body>,
    next: Next,
) -> Response {
    match authorized_session_from_request(app_state.as_ref(), &request) {
        Ok(Some(session)) => {
            request.extensions_mut().insert(session);
            next.run(request).await
        }
        Ok(None) => unauthorized_response(app_state.as_ref(), Some("invalid_token")),
        Err(_) => unauthorized_response(app_state.as_ref(), Some("server_error")),
    }
}

pub(crate) async fn protected_resource_metadata_handler(
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    Json(build_protected_resource_metadata(app_state.as_ref()))
}

pub(crate) async fn authorization_server_metadata_handler(
    State(app_state): State<Arc<AppState>>,
) -> impl IntoResponse {
    Json(build_authorization_server_metadata(app_state.as_ref()))
}

pub(crate) async fn authorize_handler(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<AuthorizationRequestQuery>,
) -> impl IntoResponse {
    match start_authorization(app_state.as_ref(), query) {
        Ok(AuthorizationStart::Redirect(redirect)) => {
            Redirect::temporary(&redirect).into_response()
        }
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn token_handler(
    State(app_state): State<Arc<AppState>>,
    Form(form): Form<TokenRequestForm>,
) -> impl IntoResponse {
    match exchange_authorization_code(app_state.as_ref(), form).await {
        Ok(response) => (StatusCode::OK, Json(response)).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn register_client_handler(
    State(app_state): State<Arc<AppState>>,
    Json(request): Json<DynamicClientRegistrationRequest>,
) -> impl IntoResponse {
    match register_client(app_state.as_ref(), request) {
        Ok(response) => (StatusCode::CREATED, Json(response)).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error.to_string()).into_response(),
    }
}

pub(crate) async fn maybe_complete_mcp_authorization(
    app_state: &AppState,
    query: &OAuthCallbackQuery,
) -> Result<Option<Response>, OAuthCallbackFailure> {
    let Some(state) = query.state.as_deref() else {
        return Ok(None);
    };
    let has_pending = app_state
        .has_pending_authorization(state)
        .map_err(|error| OAuthCallbackFailure::internal(error.to_string()))?;

    if !has_pending {
        return Ok(None);
    }

    let pending = app_state
        .take_pending_authorization(state)
        .map_err(|error| OAuthCallbackFailure::internal(error.to_string()))?
        .ok_or_else(|| {
            OAuthCallbackFailure::bad_request("保留中の MCP authorization request が見つかりません")
        })?;

    if let Some(error) = query.error.as_deref() {
        let description = query.error_description.as_deref().unwrap_or_default();
        let suffix = if description.is_empty() {
            String::new()
        } else {
            format!(": {description}")
        };
        return Err(OAuthCallbackFailure::bad_request(format!(
            "Gyazo OAuth がエラーを返しました ({error}{suffix})"
        )));
    }

    let code = query.code.as_deref().ok_or_else(|| {
        OAuthCallbackFailure::bad_request("callback に Gyazo authorization code が含まれていません")
    })?;

    let token = exchange_code_for_token(app_state, code)
        .await
        .map_err(|error| OAuthCallbackFailure::bad_gateway(error.to_string()))?;

    let redirect_uri = pending.redirect_uri.clone();
    let client_state = pending.state.clone();
    let authorization_code = issue_authorization_code(app_state, pending, token.access_token)
        .map_err(|error| OAuthCallbackFailure::internal(error.to_string()))?;
    let redirect_uri =
        build_client_redirect_url(&redirect_uri, &authorization_code, client_state.as_deref());

    Ok(Some(Redirect::temporary(&redirect_uri).into_response()))
}

fn start_authorization(
    app_state: &AppState,
    query: AuthorizationRequestQuery,
) -> Result<AuthorizationStart> {
    let pending = validate_authorization_request(app_state, query)?;

    if app_state.has_backend_api_credential()? {
        let backend_access_token = app_state
            .resolve_backend_access_token()?
            .ok_or_else(|| anyhow!("Gyazo backend access token が見つかりません"))?;
        let code = issue_authorization_code(app_state, pending.clone(), backend_access_token)?;
        let redirect =
            build_client_redirect_url(&pending.redirect_uri, &code, pending.state.as_deref());
        return Ok(AuthorizationStart::Redirect(redirect));
    }

    let gyazo_state = uuid::Uuid::new_v4().to_string();
    app_state.insert_pending_authorization(gyazo_state.clone(), pending)?;
    let redirect = build_gyazo_authorize_url(app_state, &gyazo_state)?;

    Ok(AuthorizationStart::Redirect(redirect))
}

fn validate_authorization_request(
    app_state: &AppState,
    query: AuthorizationRequestQuery,
) -> Result<PendingAuthorizationRequest> {
    let response_type = query
        .response_type
        .as_deref()
        .ok_or_else(|| anyhow!("response_type が必要です"))?;
    if response_type != "code" {
        bail!("response_type には code のみ指定できます");
    }

    let client_id = query
        .client_id
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("client_id が必要です"))?;
    let redirect_uri = query
        .redirect_uri
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("redirect_uri が必要です"))?;
    let registered_client = app_state
        .registered_client(&client_id)?
        .ok_or_else(|| anyhow!("client_id が登録されていません"))?;
    if !registered_client
        .redirect_uris
        .iter()
        .any(|uri| uri == &redirect_uri)
    {
        bail!("redirect_uri が登録内容と一致しません");
    }
    let code_challenge = query
        .code_challenge
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("code_challenge が必要です"))?;
    let code_challenge_method = query
        .code_challenge_method
        .unwrap_or_else(|| "plain".to_string());

    if code_challenge_method != "S256" {
        bail!("code_challenge_method には S256 を指定してください");
    }

    if let Some(resource) = query.resource.as_deref()
        && resource != app_state.runtime_config().mcp_url()
    {
        bail!(
            "resource には {} を指定してください",
            app_state.runtime_config().mcp_url()
        );
    }

    Ok(PendingAuthorizationRequest {
        client_id,
        redirect_uri,
        state: query.state,
        code_challenge,
        resource: query.resource,
        requested_scope: query.scope,
    })
}

fn issue_authorization_code(
    app_state: &AppState,
    pending: PendingAuthorizationRequest,
    backend_access_token: String,
) -> Result<String> {
    let grant = AuthorizationCodeGrant {
        client_id: pending.client_id,
        redirect_uri: pending.redirect_uri,
        code_challenge: pending.code_challenge,
        resource: pending.resource,
        scope: normalize_scope(pending.requested_scope.as_deref()),
        backend_access_token,
    };

    app_state.issue_authorization_code(grant)
}

async fn exchange_authorization_code(
    app_state: &AppState,
    form: TokenRequestForm,
) -> Result<TokenResponse> {
    let grant_type = form
        .grant_type
        .as_deref()
        .ok_or_else(|| anyhow!("grant_type が必要です"))?;
    if grant_type != "authorization_code" {
        bail!("grant_type には authorization_code のみ指定できます");
    }

    let code = form
        .code
        .as_deref()
        .ok_or_else(|| anyhow!("code が必要です"))?;
    let client_id = form
        .client_id
        .as_deref()
        .ok_or_else(|| anyhow!("client_id が必要です"))?;
    let registered_client = app_state
        .registered_client(client_id)?
        .ok_or_else(|| anyhow!("client_id が登録されていません"))?;
    let redirect_uri = form
        .redirect_uri
        .as_deref()
        .ok_or_else(|| anyhow!("redirect_uri が必要です"))?;
    if !registered_client
        .redirect_uris
        .iter()
        .any(|registered| registered == redirect_uri)
    {
        bail!("redirect_uri が登録内容と一致しません");
    }
    let code_verifier = form
        .code_verifier
        .as_deref()
        .ok_or_else(|| anyhow!("code_verifier が必要です"))?;

    let grant = app_state
        .take_authorization_code(code)?
        .ok_or_else(|| anyhow!("authorization code が見つからないか、すでに使用されています"))?;

    if grant.client_id != client_id {
        bail!("client_id が一致しません");
    }

    if grant.redirect_uri != redirect_uri {
        bail!("redirect_uri が一致しません");
    }

    if let Some(resource) = form.resource.as_deref()
        && Some(resource) != grant.resource.as_deref()
        && resource != app_state.runtime_config().mcp_url()
    {
        bail!("resource が一致しません");
    }

    verify_pkce(code_verifier, &grant.code_challenge)?;

    let gyazo_user = fetch_authenticated_user(&grant.backend_access_token).await?;
    let access_token = app_state.issue_access_token(AccessTokenRecord {
        backend_access_token: grant.backend_access_token,
        gyazo_user,
    })?;

    Ok(TokenResponse {
        access_token,
        token_type: "Bearer".to_string(),
        scope: grant.scope,
    })
}

fn unauthorized_response(app_state: &AppState, error: Option<&str>) -> Response {
    let metadata_url = app_state.runtime_config().protected_resource_metadata_url();
    let mut response = (
        StatusCode::UNAUTHORIZED,
        "/mcp には Bearer token が必要です。先にこのサーバーに対して MCP login を実行してください。",
    )
        .into_response();

    let mut header_value =
        format!(r#"Bearer resource_metadata="{metadata_url}", scope="{REQUIRED_SCOPE}""#);
    if let Some(error) = error {
        header_value.push_str(&format!(r#", error="{error}""#));
    }

    if let Ok(value) = HeaderValue::from_str(&header_value) {
        response.headers_mut().insert(WWW_AUTHENTICATE, value);
    }

    response
}

pub(crate) fn authorized_session_from_request(
    app_state: &AppState,
    request: &Request<Body>,
) -> Result<Option<AuthorizedSession>> {
    let Some(token) = extract_bearer_token(request.headers()) else {
        return Ok(None);
    };

    app_state.authorized_session(token)
}

pub(crate) fn authorized_session_from_parts(
    app_state: &AppState,
    parts: &Parts,
) -> Result<Option<AuthorizedSession>> {
    let Some(token) = extract_bearer_token(&parts.headers) else {
        return Ok(None);
    };

    app_state.authorized_session(token)
}

fn extract_bearer_token(headers: &axum::http::HeaderMap) -> Option<&str> {
    let value = headers.get(AUTHORIZATION)?.to_str().ok()?;
    value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
}

fn build_protected_resource_metadata(app_state: &AppState) -> ProtectedResourceMetadata {
    let runtime_config = app_state.runtime_config();

    ProtectedResourceMetadata {
        resource: runtime_config.mcp_url(),
        authorization_servers: vec![runtime_config.authorization_server_issuer()],
        scopes_supported: vec![REQUIRED_SCOPE.to_string()],
    }
}

fn build_authorization_server_metadata(app_state: &AppState) -> AuthorizationServerMetadata {
    let runtime_config = app_state.runtime_config();

    AuthorizationServerMetadata {
        issuer: runtime_config.authorization_server_issuer(),
        authorization_endpoint: runtime_config.authorization_endpoint_url(),
        token_endpoint: runtime_config.token_endpoint_url(),
        registration_endpoint: runtime_config.registration_endpoint_url(),
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string()],
        token_endpoint_auth_methods_supported: vec!["none".to_string()],
        code_challenge_methods_supported: vec!["S256".to_string()],
    }
}

fn register_client(
    app_state: &AppState,
    request: DynamicClientRegistrationRequest,
) -> Result<DynamicClientRegistrationResponse> {
    let redirect_uris = request
        .redirect_uris
        .filter(|uris| !uris.is_empty())
        .ok_or_else(|| anyhow!("redirect_uris が必要です"))?;

    if let Some(method) = request.token_endpoint_auth_method.as_deref()
        && method != "none"
    {
        bail!("token_endpoint_auth_method には none のみ指定できます");
    }

    if let Some(grant_types) = request.grant_types.as_ref()
        && !grant_types
            .iter()
            .any(|grant| grant == "authorization_code")
    {
        bail!("grant_types には authorization_code が必要です");
    }

    if let Some(response_types) = request.response_types.as_ref()
        && !response_types.iter().any(|response| response == "code")
    {
        bail!("response_types には code が必要です");
    }

    let client_name = request.client_name.filter(|name| !name.trim().is_empty());
    let client_id = app_state.register_client(RegisteredClient {
        redirect_uris: redirect_uris.clone(),
    })?;

    Ok(DynamicClientRegistrationResponse {
        client_id,
        redirect_uris,
        client_name,
        grant_types: vec!["authorization_code".to_string()],
        response_types: vec!["code".to_string()],
        token_endpoint_auth_method: "none".to_string(),
    })
}

fn build_client_redirect_url(base: &str, code: &str, state: Option<&str>) -> String {
    let separator = if base.contains('?') { '&' } else { '?' };
    let mut url = format!("{base}{separator}code={}", percent_encode(code));

    if let Some(state) = state {
        url.push_str("&state=");
        url.push_str(&percent_encode(state));
    }

    url
}

fn verify_pkce(code_verifier: &str, code_challenge: &str) -> Result<()> {
    let mut hasher = Sha256::new();
    hasher.update(code_verifier.as_bytes());
    let digest = hasher.finalize();
    let actual = URL_SAFE_NO_PAD.encode(digest);

    if actual != code_challenge {
        bail!("code_verifier が一致しません");
    }

    Ok(())
}

fn normalize_scope(requested_scope: Option<&str>) -> String {
    match requested_scope {
        Some(scope) if !scope.trim().is_empty() => scope.to_string(),
        _ => REQUIRED_SCOPE.to_string(),
    }
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

enum AuthorizationStart {
    Redirect(String),
}

#[cfg(test)]
mod tests {
    use super::{build_client_redirect_url, verify_pkce};

    #[test]
    fn appends_code_and_state_to_redirect_uri() {
        let url = build_client_redirect_url(
            "http://127.0.0.1:3000/callback",
            "code-123",
            Some("state-456"),
        );

        assert_eq!(
            url,
            "http://127.0.0.1:3000/callback?code=code-123&state=state-456"
        );
    }

    #[test]
    fn accepts_matching_s256_pkce() {
        verify_pkce(
            "dBjftJeZ4CVP-mB92K27uhbUJU1p1r_wW1gFWFOEjXk",
            "E9Melhoa2OwvFrEMTJguCHaoeK1t8URWbuGJSstw-cM",
        )
        .unwrap();
    }
}
