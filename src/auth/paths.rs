use std::{path::PathBuf, sync::OnceLock};

static CONFIG_DIR_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

/// CLI の `--config-dir` オプションで上書きされた場合にセットする。
/// main の最初期に一度だけ呼ぶ。
pub(crate) fn set_config_dir_override(path: PathBuf) {
    let _ = CONFIG_DIR_OVERRIDE.set(path);
}

pub(crate) fn has_config_dir_override() -> bool {
    CONFIG_DIR_OVERRIDE.get().is_some()
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_falls_back_to_dirs_config_dir() {
        // GYAZO_MCP_CONFIG_DIR が未設定かつ CLI override がない場合、
        // dirs::config_dir() ベースのパスを返す
        let dir = dirs::config_dir().map(|d| d.join("gyazo-mcp-server"));
        // OnceLock が他のテストでセットされていなければ一致する
        if !has_config_dir_override() {
            // 環境変数もなければデフォルト
            if std::env::var("GYAZO_MCP_CONFIG_DIR").is_err() {
                assert_eq!(config_dir(), dir);
            }
        }
    }

    #[test]
    fn derived_paths_are_under_config_dir() {
        if let Some(base) = config_dir() {
            assert_eq!(env_file_path(), Some(base.join(".env")));
            assert_eq!(token_file_path(), Some(base.join("token.toml")));
            assert_eq!(mcp_session_file_path(), Some(base.join("mcp_sessions.toml")));
            assert_eq!(config_file_path(), Some(base.join("config.toml")));
        }
    }
}
