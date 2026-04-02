use std::sync::Mutex;

use anyhow::{Context, Result, anyhow};

use crate::{
    auth::{
        config::AuthConfig,
        state::AuthState,
        token_store::{StoredToken, save_token},
    },
    runtime_config::RuntimeConfig,
};

pub(crate) struct AppState {
    auth_config: AuthConfig,
    auth_state: Mutex<AuthState>,
    oauth_session: Mutex<OAuthSessionState>,
    runtime_config: RuntimeConfig,
}

#[derive(Debug, Default)]
struct OAuthSessionState {
    pending_state: Option<String>,
}

impl AppState {
    pub(crate) fn new(runtime_config: RuntimeConfig) -> Result<Self> {
        Ok(Self {
            auth_config: AuthConfig::from_env(),
            auth_state: Mutex::new(AuthState::load()?),
            oauth_session: Mutex::new(OAuthSessionState::default()),
            runtime_config,
        })
    }

    pub(crate) fn auth_config(&self) -> &AuthConfig {
        &self.auth_config
    }

    pub(crate) fn auth_state_snapshot(&self) -> Result<AuthState> {
        let guard = self
            .auth_state
            .lock()
            .map_err(|_| anyhow!("auth state lock is poisoned"))?;
        Ok(guard.clone())
    }

    pub(crate) fn runtime_config(&self) -> &RuntimeConfig {
        &self.runtime_config
    }

    pub(crate) fn save_oauth_token(&self, token: StoredToken) -> Result<()> {
        let mut guard = self
            .auth_state
            .lock()
            .map_err(|_| anyhow!("auth state lock is poisoned"))?;
        let path = guard
            .token_file_path
            .clone()
            .context("token file path is unavailable")?;

        save_token(&path, &token)?;
        guard.stored_token = Some(token);

        Ok(())
    }

    pub(crate) fn set_pending_oauth_state(&self, state: String) -> Result<()> {
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        guard.pending_state = Some(state);
        Ok(())
    }

    pub(crate) fn take_pending_oauth_state(&self) -> Result<Option<String>> {
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        Ok(guard.pending_state.take())
    }
}
