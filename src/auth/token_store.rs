use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredToken {
    pub(crate) access_token: String,
}

pub(crate) fn load_token(path: &Path) -> Result<Option<StoredToken>> {
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(path)
        .with_context(|| format!("token file を読み取れませんでした: {}", path.display()))?;
    let token = toml::from_str(&raw)
        .with_context(|| format!("token file を解析できませんでした: {}", path.display()))?;

    Ok(Some(token))
}

pub(crate) fn save_token(path: &Path, token: &StoredToken) -> Result<()> {
    let raw = toml::to_string(token).context("token file をシリアライズできませんでした")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "token directory を作成できませんでした: {}",
                parent.display()
            )
        })?;
    }

    fs::write(path, raw)
        .with_context(|| format!("token file に書き込めませんでした: {}", path.display()))
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{StoredToken, load_token, save_token};

    #[test]
    fn saves_and_loads_token_toml() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("gyazo-mcp-server-test-{unique}"));
        let path = dir.join("token.toml");
        let token = StoredToken {
            access_token: "test-token".to_string(),
        };
        fs::create_dir_all(&dir).unwrap();
        save_token(&path, &token).unwrap();
        let loaded = load_token(&path).unwrap();

        assert_eq!(loaded, Some(token));

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
    }
}
