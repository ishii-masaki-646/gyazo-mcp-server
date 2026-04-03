use std::{fs, io::IsTerminal, path::PathBuf};

use anyhow::{Context, Result, bail};
use inquire::{Password, Text};

use super::paths;

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
    std::env::var(key)
        .ok()
        .filter(|value| !value.trim().is_empty())
}

const VALID_ENV_KEYS: &[&str] = &[
    "GYAZO_MCP_OAUTH_CLIENT_ID",
    "GYAZO_MCP_OAUTH_CLIENT_SECRET",
    "GYAZO_MCP_PERSONAL_ACCESS_TOKEN",
];

const SECRET_KEYS: &[&str] = &[
    "GYAZO_MCP_OAUTH_CLIENT_SECRET",
    "GYAZO_MCP_PERSONAL_ACCESS_TOKEN",
];

fn should_mask() -> bool {
    std::io::stdout().is_terminal()
}

fn mask_secret(value: &str) -> String {
    if value.len() <= 4 {
        "****".to_string()
    } else {
        format!("****...{}", &value[value.len() - 4..])
    }
}

fn load_env_entries(path: &std::path::Path) -> Result<Vec<(String, String)>> {
    if !path.exists() {
        return Ok(Vec::new());
    }

    let contents = fs::read_to_string(path)
        .with_context(|| format!(".env を読み取れませんでした: {}", path.display()))?;

    Ok(contents
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .filter_map(|line| {
            let (key, value) = line.split_once('=')?;
            Some((key.trim().to_string(), value.trim().to_string()))
        })
        .collect())
}

pub(crate) fn show_env() -> Result<()> {
    let path = paths::env_file_path()
        .ok_or_else(|| anyhow::anyhow!("設定ディレクトリを特定できませんでした"))?;
    let entries = load_env_entries(&path)?;

    let mask = should_mask();
    for key in VALID_ENV_KEYS {
        let file_value = entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
            .filter(|v| !v.trim().is_empty());

        match file_value {
            Some(value) => {
                if mask && SECRET_KEYS.contains(key) {
                    println!("{key} = \"{}\" (set)", mask_secret(value));
                } else {
                    println!("{key} = \"{value}\" (set)");
                }
            }
            None => {
                println!("{key} = (not set)");
            }
        }
    }

    Ok(())
}

pub(crate) fn get_env(key: &str) -> Result<()> {
    if !VALID_ENV_KEYS.contains(&key) {
        bail!(
            "不明な環境変数: {key}\n有効なキー: {}",
            VALID_ENV_KEYS.join(", ")
        );
    }

    let path = paths::env_file_path()
        .ok_or_else(|| anyhow::anyhow!("設定ディレクトリを特定できませんでした"))?;
    let entries = load_env_entries(&path)?;

    let value = entries
        .iter()
        .find(|(k, _)| k == key)
        .map(|(_, v)| v.as_str())
        .filter(|v| !v.trim().is_empty());

    match value {
        Some(v) => {
            if should_mask() && SECRET_KEYS.contains(&key) {
                println!("{}", mask_secret(v));
            } else {
                println!("{v}");
            }
        }
        None => println!("(not set)"),
    }
    Ok(())
}

pub(crate) fn init_env() -> Result<()> {
    let path = paths::env_file_path()
        .ok_or_else(|| anyhow::anyhow!("設定ディレクトリを特定できませんでした"))?;
    let entries = load_env_entries(&path)?;

    let find_entry = |key: &str| -> Option<String> {
        entries
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .filter(|v| !v.trim().is_empty())
    };
    let current_id = find_entry("GYAZO_MCP_OAUTH_CLIENT_ID");
    let has_secret = find_entry("GYAZO_MCP_OAUTH_CLIENT_SECRET").is_some();
    let has_pat = find_entry("GYAZO_MCP_PERSONAL_ACCESS_TOKEN").is_some();

    let id_default = current_id.as_deref().unwrap_or("");
    let id_hint = if current_id.is_some() {
        "空 Enter で現在の値を維持。"
    } else {
        "空 Enter で未設定のまま続行。"
    };
    let client_id = Text::new(&format!("GYAZO_MCP_OAUTH_CLIENT_ID [{id_default}]:"))
        .with_help_message(&format!(
            "Gyazo 開発者ページで発行した OAuth アプリの Client ID。{id_hint}",
        ))
        .prompt()?;
    let client_id = if client_id.is_empty() {
        id_default.to_string()
    } else {
        client_id
    };

    let secret_hint = if has_secret {
        "空 Enter で現在の値を維持。"
    } else {
        "空 Enter で未設定のまま続行。"
    };
    let client_secret = Password::new("GYAZO_MCP_OAUTH_CLIENT_SECRET:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .with_help_message(&format!(
            "Gyazo 開発者ページで発行した OAuth アプリの Client Secret。{secret_hint}",
        ))
        .without_confirmation()
        .prompt()?;

    let pat_hint = if has_pat {
        "空 Enter で現在の値を維持。"
    } else {
        "空 Enter で未設定のまま続行。"
    };
    let pat = Password::new("GYAZO_MCP_PERSONAL_ACCESS_TOKEN:")
        .with_display_mode(inquire::PasswordDisplayMode::Masked)
        .with_help_message(&format!(
            "Gyazo の Personal Access Token。OAuth を使わない場合に設定。{pat_hint}",
        ))
        .without_confirmation()
        .prompt()?;

    if !client_id.is_empty() && client_id != id_default {
        set_env("GYAZO_MCP_OAUTH_CLIENT_ID", &client_id)?;
    }
    if !client_secret.is_empty() {
        set_env("GYAZO_MCP_OAUTH_CLIENT_SECRET", &client_secret)?;
    }
    if !pat.is_empty() {
        set_env("GYAZO_MCP_PERSONAL_ACCESS_TOKEN", &pat)?;
    }

    println!("\n.env の初期設定が完了しました");
    Ok(())
}

/// デフォルト位置の .env から GYAZO_MCP_CONFIG_DIR を読み取る。
/// fresh process の bootstrap 用。load_env_files() より前に呼ぶ。
pub(crate) fn read_config_dir_from_default_env() -> Option<String> {
    let path = default_env_file_path()?;
    let contents = std::fs::read_to_string(path).ok()?;
    contents
        .lines()
        .find(|line| line.trim_start().starts_with("GYAZO_MCP_CONFIG_DIR="))
        .and_then(|line| line.split_once('='))
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// デフォルトの設定ディレクトリ (dirs::config_dir()) の .env パスを返す。
/// config_dir の永続化先として使う。config_dir の値で .env の場所が変わると
/// 次回起動時にたどり着けなくなるため、常にデフォルト位置に書く。
pub(crate) fn default_env_file_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("gyazo-mcp-server").join(".env"))
}

pub(crate) fn unset_env(key: &str) -> Result<()> {
    if !VALID_ENV_KEYS.contains(&key) {
        bail!(
            "不明な環境変数: {key}\n有効なキー: {}",
            VALID_ENV_KEYS.join(", ")
        );
    }

    let path = paths::env_file_path()
        .ok_or_else(|| anyhow::anyhow!("設定ディレクトリを特定できませんでした"))?;

    if !path.exists() {
        println!("{key} は設定されていません");
        return Ok(());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!(".env を読み取れませんでした: {}", path.display()))?;

    let target_prefix = format!("{key}=");
    let original_len = contents.lines().count();
    let lines: Vec<&str> = contents
        .lines()
        .filter(|line| !line.trim_start().starts_with(&target_prefix))
        .collect();

    if lines.len() == original_len {
        println!("{key} は設定されていません");
        return Ok(());
    }

    let mut output = lines.join("\n");
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }

    fs::write(&path, output)
        .with_context(|| format!(".env に書き込めませんでした: {}", path.display()))?;

    println!("{key} を .env から削除しました");
    Ok(())
}

pub(crate) fn set_env(key: &str, value: &str) -> Result<()> {
    if !VALID_ENV_KEYS.contains(&key) {
        bail!(
            "不明な環境変数: {key}\n有効なキー: {}",
            VALID_ENV_KEYS.join(", ")
        );
    }

    let path = paths::env_file_path()
        .ok_or_else(|| anyhow::anyhow!("設定ディレクトリを特定できませんでした"))?;

    let contents = if path.exists() {
        fs::read_to_string(&path)
            .with_context(|| format!(".env を読み取れませんでした: {}", path.display()))?
    } else {
        String::new()
    };

    let target_prefix = format!("{key}=");
    let mut found = false;
    let mut lines: Vec<String> = contents
        .lines()
        .map(|line| {
            if line.trim_start().starts_with(&target_prefix) {
                found = true;
                format!("{key}={value}")
            } else {
                line.to_string()
            }
        })
        .collect();

    if !found {
        lines.push(format!("{key}={value}"));
    }

    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "設定ディレクトリを作成できませんでした: {}",
                parent.display()
            )
        })?;
    }

    fs::write(&path, output)
        .with_context(|| format!(".env に書き込めませんでした: {}", path.display()))?;

    let display_value = if SECRET_KEYS.contains(&key) {
        mask_secret(value)
    } else {
        value.to_string()
    };
    println!("{key} = \"{display_value}\" を .env に保存しました");
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;

    fn temp_env_path() -> std::path::PathBuf {
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("gyazo-mcp-env-test-{unique}"));
        fs::create_dir_all(&dir).unwrap();
        dir.join(".env")
    }

    #[test]
    fn config_dir_is_not_a_valid_env_key() {
        // GYAZO_MCP_CONFIG_DIR は env コマンドではなく config set config_dir で管理する。
        // env set 経由だと bootstrap を迂回して custom .env に書いてしまうため。
        assert!(
            !VALID_ENV_KEYS.contains(&"GYAZO_MCP_CONFIG_DIR"),
            "GYAZO_MCP_CONFIG_DIR は VALID_ENV_KEYS に含めてはならない"
        );
    }

    #[test]
    fn set_env_rejects_config_dir() {
        let result = set_env("GYAZO_MCP_CONFIG_DIR", "/tmp/test");
        assert!(result.is_err());
    }

    #[test]
    fn unset_env_rejects_config_dir() {
        let result = unset_env("GYAZO_MCP_CONFIG_DIR");
        assert!(result.is_err());
    }

    #[test]
    fn get_env_rejects_config_dir() {
        let result = get_env("GYAZO_MCP_CONFIG_DIR");
        assert!(result.is_err());
    }

    #[test]
    fn load_env_entries_extracts_config_dir() {
        let path = temp_env_path();
        fs::write(&path, "GYAZO_MCP_CONFIG_DIR=/custom/path\nOTHER=value\n").unwrap();

        let entries = load_env_entries(&path).unwrap();
        let config_dir = entries
            .iter()
            .find(|(k, _)| k == "GYAZO_MCP_CONFIG_DIR")
            .map(|(_, v)| v.as_str());

        assert_eq!(config_dir, Some("/custom/path"));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().unwrap());
    }

    #[test]
    fn mask_secret_short_value() {
        assert_eq!(mask_secret("abc"), "****");
    }

    #[test]
    fn mask_secret_long_value() {
        assert_eq!(mask_secret("abcdefgh"), "****...efgh");
    }

    #[test]
    fn load_env_entries_parses_key_value_pairs() {
        let path = temp_env_path();
        fs::write(&path, "KEY1=value1\n# comment\nKEY2=value2\n").unwrap();

        let entries = load_env_entries(&path).unwrap();

        assert_eq!(
            entries,
            vec![
                ("KEY1".to_string(), "value1".to_string()),
                ("KEY2".to_string(), "value2".to_string()),
            ]
        );

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(path.parent().unwrap());
    }

    #[test]
    fn load_env_entries_returns_empty_for_missing_file() {
        let path = std::path::PathBuf::from("/tmp/nonexistent-gyazo-test/.env");

        let entries = load_env_entries(&path).unwrap();

        assert!(entries.is_empty());
    }
}
