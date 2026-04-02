use std::sync::Arc;

use axum::{
    Json,
    body::Body,
    extract::State,
    http::{
        HeaderValue, Request, StatusCode,
        header::{AUTHORIZATION, WWW_AUTHENTICATE},
    },
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::Serialize;

use crate::app_state::AppState;

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
    response_types_supported: Vec<String>,
    grant_types_supported: Vec<String>,
    token_endpoint_auth_methods_supported: Vec<String>,
    code_challenge_methods_supported: Vec<String>,
}

pub(crate) async fn require_mcp_bearer_token(
    State(app_state): State<Arc<AppState>>,
    request: Request<Body>,
    next: Next,
) -> Response {
    if has_bearer_token(&request) {
        return next.run(request).await;
    }

    unauthorized_response(app_state.as_ref(), None)
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

pub(crate) async fn authorize_placeholder_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        "MCP authorization endpoint is not wired yet. Gyazo-backed authorization brokering will be added next.",
    )
}

pub(crate) async fn token_placeholder_handler() -> impl IntoResponse {
    (
        StatusCode::NOT_IMPLEMENTED,
        "MCP token endpoint is not wired yet. Gyazo-backed token issuance will be added next.",
    )
}

fn unauthorized_response(app_state: &AppState, error: Option<&str>) -> Response {
    let metadata_url = app_state.runtime_config().protected_resource_metadata_url();
    let mut response = (
        StatusCode::UNAUTHORIZED,
        "Bearer token is required for /mcp. Use MCP login against this server first.",
    )
        .into_response();

    let mut header_value = format!(
        r#"Bearer resource_metadata="{metadata_url}", scope="{REQUIRED_SCOPE}""#
    );
    if let Some(error) = error {
        header_value.push_str(&format!(r#", error="{error}""#));
    }

    if let Ok(value) = HeaderValue::from_str(&header_value) {
        response.headers_mut().insert(WWW_AUTHENTICATE, value);
    }

    response
}

fn has_bearer_token(request: &Request<Body>) -> bool {
    let Some(value) = request.headers().get(AUTHORIZATION) else {
        return false;
    };
    let Ok(value) = value.to_str() else {
        return false;
    };

    value
        .strip_prefix("Bearer ")
        .map(|token| !token.trim().is_empty())
        .unwrap_or(false)
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
        response_types_supported: vec!["code".to_string()],
        grant_types_supported: vec!["authorization_code".to_string()],
        token_endpoint_auth_methods_supported: vec!["none".to_string()],
        code_challenge_methods_supported: vec!["S256".to_string()],
    }
}
