use std::{collections::HashMap, fs, path::Path};

use anyhow::{Context, Result};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use uuid::Uuid;

use crate::{app_state::AccessTokenRecord, gyazo_api::GyazoUserProfile};

type HmacSha256 = Hmac<Sha256>;

const TOKEN_PREFIX: &str = "gmcp1";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredMcpSessionState {
    pub(crate) signing_key: String,
    pub(crate) sessions: Vec<StoredMcpSession>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct StoredMcpSession {
    pub(crate) session_id: String,
    pub(crate) backend_access_token: String,
    pub(crate) gyazo_user: GyazoUserProfile,
}

pub(crate) fn load_mcp_session_state(path: &Path) -> Result<Option<StoredMcpSessionState>> {
    if !path.exists() {
        return Ok(None);
    }

    let raw = fs::read_to_string(path).with_context(|| {
        format!(
            "MCP session file を読み取れませんでした: {}",
            path.display()
        )
    })?;
    let state = toml::from_str(&raw).with_context(|| {
        format!(
            "MCP session file を解析できませんでした: {}",
            path.display()
        )
    })?;

    Ok(Some(state))
}

pub(crate) fn save_mcp_session_state(path: &Path, state: &StoredMcpSessionState) -> Result<()> {
    let raw = toml::to_string(state).context("MCP session file をシリアライズできませんでした")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "MCP session directory を作成できませんでした: {}",
                parent.display()
            )
        })?;
    }

    fs::write(path, raw).with_context(|| {
        format!(
            "MCP session file に書き込めませんでした: {}",
            path.display()
        )
    })
}

pub(crate) fn generate_signing_key() -> Vec<u8> {
    let mut signing_key = Vec::with_capacity(32);
    signing_key.extend_from_slice(Uuid::new_v4().as_bytes());
    signing_key.extend_from_slice(Uuid::new_v4().as_bytes());
    signing_key
}

pub(crate) fn encode_signing_key(signing_key: &[u8]) -> String {
    URL_SAFE_NO_PAD.encode(signing_key)
}

pub(crate) fn decode_signing_key(encoded: &str) -> Result<Vec<u8>> {
    URL_SAFE_NO_PAD
        .decode(encoded)
        .context("MCP signing key をデコードできませんでした")
}

pub(crate) fn sign_access_token(signing_key: &[u8], session_id: &str) -> Result<String> {
    let signature = token_signature(signing_key, session_id)?;
    Ok(format!("{TOKEN_PREFIX}.{session_id}.{signature}"))
}

pub(crate) fn verify_access_token(signing_key: &[u8], token: &str) -> Result<Option<String>> {
    let mut parts = token.split('.');
    let prefix = parts.next();
    let session_id = parts.next();
    let signature = parts.next();

    if prefix != Some(TOKEN_PREFIX) || parts.next().is_some() {
        return Ok(None);
    }

    let Some(session_id) = session_id else {
        return Ok(None);
    };
    let Some(signature) = signature else {
        return Ok(None);
    };

    let expected_signature = token_signature(signing_key, session_id)?;
    if expected_signature != signature {
        return Ok(None);
    }

    Ok(Some(session_id.to_string()))
}

pub(crate) fn sessions_to_records(
    sessions: Vec<StoredMcpSession>,
) -> HashMap<String, AccessTokenRecord> {
    sessions
        .into_iter()
        .map(|session| {
            (
                session.session_id,
                AccessTokenRecord {
                    backend_access_token: session.backend_access_token,
                    gyazo_user: session.gyazo_user,
                },
            )
        })
        .collect()
}

pub(crate) fn records_to_sessions(
    records: &HashMap<String, AccessTokenRecord>,
) -> Vec<StoredMcpSession> {
    records
        .iter()
        .map(|(session_id, record)| StoredMcpSession {
            session_id: session_id.clone(),
            backend_access_token: record.backend_access_token.clone(),
            gyazo_user: record.gyazo_user.clone(),
        })
        .collect()
}

fn token_signature(signing_key: &[u8], session_id: &str) -> Result<String> {
    let mut mac =
        HmacSha256::new_from_slice(signing_key).context("HMAC signer を初期化できませんでした")?;
    mac.update(TOKEN_PREFIX.as_bytes());
    mac.update(b":");
    mac.update(session_id.as_bytes());
    let signature = mac.finalize().into_bytes();

    Ok(URL_SAFE_NO_PAD.encode(signature))
}

#[cfg(test)]
mod tests {
    use std::{
        collections::HashMap,
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{
        StoredMcpSessionState, decode_signing_key, encode_signing_key, generate_signing_key,
        load_mcp_session_state, records_to_sessions, save_mcp_session_state, sessions_to_records,
        sign_access_token, verify_access_token,
    };
    use crate::{app_state::AccessTokenRecord, gyazo_api::GyazoUserProfile};

    #[test]
    fn signs_and_verifies_access_token() {
        let signing_key = generate_signing_key();
        let token = sign_access_token(&signing_key, "session-123").unwrap();
        let verified = verify_access_token(&signing_key, &token).unwrap();

        assert_eq!(verified.as_deref(), Some("session-123"));
        assert!(
            verify_access_token(&signing_key, "gmcp1.session-123.invalid")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn saves_and_loads_mcp_session_state() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("gyazo-mcp-sessions-test-{unique}"));
        let path = dir.join("mcp_sessions.toml");
        let signing_key = generate_signing_key();
        let mut records = HashMap::new();
        records.insert(
            "session-1".to_string(),
            AccessTokenRecord {
                backend_access_token: "backend-token".to_string(),
                gyazo_user: GyazoUserProfile {
                    email: "test@example.com".to_string(),
                    name: "tester".to_string(),
                    profile_image: "https://example.com/avatar.png".to_string(),
                    uid: "user-1".to_string(),
                },
            },
        );
        let state = StoredMcpSessionState {
            signing_key: encode_signing_key(&signing_key),
            sessions: records_to_sessions(&records),
        };

        fs::create_dir_all(&dir).unwrap();
        save_mcp_session_state(&path, &state).unwrap();
        let loaded = load_mcp_session_state(&path).unwrap().unwrap();

        assert_eq!(
            decode_signing_key(&loaded.signing_key).unwrap(),
            signing_key
        );
        assert_eq!(sessions_to_records(loaded.sessions), records);

        let _ = fs::remove_file(&path);
        let _ = fs::remove_dir(&dir);
    }
}
