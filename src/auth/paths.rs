use std::{path::PathBuf, sync::OnceLock};

static CONFIG_DIR_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// CLI の `--config-dir` オプションで上書きされた場合にセットする。
/// main の最初期に一度だけ呼ぶ。
pub(crate) fn set_config_dir_override(path: PathBuf) {
    let _ = CONFIG_DIR_OVERRIDE.set(path);
}

pub(crate) fn config_dir() -> Option<PathBuf> {
    if let Some(override_dir) = CONFIG_DIR_OVERRIDE.get() {
        return Some(override_dir.clone());
    }

    if let Ok(dir) = std::env::var("GYAZO_MCP_CONFIG_DIR")
        && !dir.trim().is_empty()
    {
        return Some(PathBuf::from(dir));
    }

    dirs::config_dir().map(|dir| dir.join("gyazo-mcp-server"))
}

pub(crate) fn env_file_path() -> Option<PathBuf> {
    Some(config_dir()?.join(".env"))
}

pub(crate) fn token_file_path() -> Option<PathBuf> {
    Some(config_dir()?.join("token.toml"))
}

pub(crate) fn mcp_session_file_path() -> Option<PathBuf> {
    Some(config_dir()?.join("mcp_sessions.toml"))
}

pub(crate) fn config_file_path() -> Option<PathBuf> {
    Some(config_dir()?.join("config.toml"))
}
