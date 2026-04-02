#[derive(Debug, Clone, Default)]
pub(crate) struct AuthConfig {
    oauth_client_id: Option<String>,
    oauth_client_secret: Option<String>,
    personal_access_token: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct OAuthCredentials {
    pub(crate) client_id: String,
    pub(crate) client_secret: String,
}

impl AuthConfig {
    pub(crate) fn from_env() -> Self {
        Self {
            oauth_client_id: read_env("GYAZO_MCP_OAUTH_CLIENT_ID"),
            oauth_client_secret: read_env("GYAZO_MCP_OAUTH_CLIENT_SECRET"),
            personal_access_token: read_env("GYAZO_MCP_PERSONAL_ACCESS_TOKEN"),
        }
    }

    pub(crate) fn has_personal_access_token(&self) -> bool {
        self.personal_access_token.is_some()
    }

    pub(crate) fn personal_access_token(&self) -> Option<String> {
        self.personal_access_token.clone()
    }

    pub(crate) fn oauth_credentials(&self) -> Option<OAuthCredentials> {
        Some(OAuthCredentials {
            client_id: self.oauth_client_id.clone()?,
            client_secret: self.oauth_client_secret.clone()?,
        })
    }
}

fn read_env(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|value| !value.trim().is_empty())
}
