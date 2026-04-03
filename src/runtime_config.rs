use std::{fs, net::{IpAddr, Ipv4Addr, SocketAddr}};

use anyhow::{Context, Result, bail};
use serde::Deserialize;
use tracing_subscriber::EnvFilter;

use crate::auth::paths;

#[derive(Debug, Clone, Deserialize, Default)]
struct RuntimeConfigFile {
    tcp_port: Option<u16>,
    oauth_callback_path: Option<String>,
    rust_log: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfig {
    tcp_port: u16,
    oauth_callback_path: String,
    rust_log: Option<String>,
}

impl RuntimeConfig {
    pub(crate) fn load() -> Result<Self> {
        let file_config = load_runtime_config_file()?;

        let tcp_port = std::env::var("GYAZO_MCP_TCP_PORT")
            .ok()
            .map(|value| value.parse::<u16>())
            .transpose()?
            .or(file_config.tcp_port)
            .unwrap_or(18449);
        let oauth_callback_path = std::env::var("GYAZO_MCP_OAUTH_CALLBACK_PATH")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or(file_config.oauth_callback_path)
            .unwrap_or_else(|| "/oauth/callback".to_string());
        let rust_log = std::env::var("RUST_LOG")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or(file_config.rust_log);

        if !oauth_callback_path.starts_with('/') {
            bail!("GYAZO_MCP_OAUTH_CALLBACK_PATH must start with '/'");
        }
        if let Some(rust_log) = &rust_log {
            EnvFilter::try_new(rust_log)
                .with_context(|| format!("RUST_LOG / rust_log を解釈できませんでした: {rust_log}"))?;
        }

        Ok(Self {
            tcp_port,
            oauth_callback_path,
            rust_log,
        })
    }

    pub(crate) fn tracing_env_filter(&self) -> EnvFilter {
        self.rust_log
            .as_deref()
            .map(EnvFilter::new)
            .unwrap_or_else(|| EnvFilter::new("gyazo_mcp_server=info,rmcp=info"))
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

fn load_runtime_config_file() -> Result<RuntimeConfigFile> {
    let Some(path) = paths::config_dir().map(|dir| dir.join("config.toml")) else {
        return Ok(RuntimeConfigFile::default());
    };

    if !path.exists() {
        return Ok(RuntimeConfigFile::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("config.toml を読み取れませんでした: {}", path.display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("config.toml を解析できませんでした: {}", path.display()))
}
