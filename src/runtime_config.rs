use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use anyhow::{Result, bail};

#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfig {
    tcp_port: u16,
    oauth_callback_path: String,
}

impl RuntimeConfig {
    pub(crate) fn from_env() -> Result<Self> {
        let tcp_port = std::env::var("GYAZO_MCP_TCP_PORT")
            .ok()
            .map(|value| value.parse::<u16>())
            .transpose()?
            .unwrap_or(18449);
        let oauth_callback_path = std::env::var("GYAZO_MCP_OAUTH_CALLBACK_PATH")
            .unwrap_or_else(|_| "/oauth/callback".to_string());

        if !oauth_callback_path.starts_with('/') {
            bail!("GYAZO_MCP_OAUTH_CALLBACK_PATH must start with '/'");
        }

        Ok(Self {
            tcp_port,
            oauth_callback_path,
        })
    }

    pub(crate) fn bind_address(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), self.tcp_port)
    }

    pub(crate) fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.tcp_port)
    }

    pub(crate) fn mcp_path(&self) -> &'static str {
        "/mcp"
    }

    pub(crate) fn protected_resource_metadata_root_path(&self) -> &'static str {
        "/.well-known/oauth-protected-resource"
    }

    pub(crate) fn protected_resource_metadata_path(&self) -> String {
        format!(
            "{}/{}",
            self.protected_resource_metadata_root_path(),
            self.mcp_path().trim_start_matches('/')
        )
    }

    pub(crate) fn authorization_server_metadata_path(&self) -> &'static str {
        "/.well-known/oauth-authorization-server"
    }

    pub(crate) fn authorization_endpoint_path(&self) -> &'static str {
        "/authorize"
    }

    pub(crate) fn token_endpoint_path(&self) -> &'static str {
        "/token"
    }

    pub(crate) fn registration_endpoint_path(&self) -> &'static str {
        "/register"
    }

    pub(crate) fn oauth_start_path(&self) -> &'static str {
        "/oauth/start"
    }

    pub(crate) fn oauth_callback_path(&self) -> &str {
        &self.oauth_callback_path
    }

    pub(crate) fn mcp_url(&self) -> String {
        format!("{}{}", self.base_url(), self.mcp_path())
    }

    pub(crate) fn protected_resource_metadata_url(&self) -> String {
        format!(
            "{}{}",
            self.base_url(),
            self.protected_resource_metadata_path()
        )
    }

    pub(crate) fn authorization_server_issuer(&self) -> String {
        self.base_url()
    }

    pub(crate) fn authorization_server_metadata_url(&self) -> String {
        format!(
            "{}{}",
            self.base_url(),
            self.authorization_server_metadata_path()
        )
    }

    pub(crate) fn authorization_endpoint_url(&self) -> String {
        format!("{}{}", self.base_url(), self.authorization_endpoint_path())
    }

    pub(crate) fn token_endpoint_url(&self) -> String {
        format!("{}{}", self.base_url(), self.token_endpoint_path())
    }

    pub(crate) fn registration_endpoint_url(&self) -> String {
        format!("{}{}", self.base_url(), self.registration_endpoint_path())
    }

    pub(crate) fn oauth_start_url(&self) -> String {
        format!("{}{}", self.base_url(), self.oauth_start_path())
    }

    pub(crate) fn oauth_callback_url(&self) -> String {
        format!("{}{}", self.base_url(), self.oauth_callback_path())
    }
}
