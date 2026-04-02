use std::{collections::HashMap, sync::Mutex};

use anyhow::{Context, Result, anyhow};
use uuid::Uuid;

use crate::{
    auth::{
        config::AuthConfig,
        state::AuthState,
        token_store::{StoredToken, save_token},
    },
    gyazo_api::GyazoUserProfile,
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
    pending_direct_login_state: Option<String>,
    registered_clients: HashMap<String, RegisteredClient>,
    pending_authorizations: HashMap<String, PendingAuthorizationRequest>,
    authorization_codes: HashMap<String, AuthorizationCodeGrant>,
    access_tokens: HashMap<String, AccessTokenRecord>,
}

#[derive(Debug, Clone)]
pub(crate) struct RegisteredClient {
    pub(crate) redirect_uris: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct PendingAuthorizationRequest {
    pub(crate) client_id: String,
    pub(crate) redirect_uri: String,
    pub(crate) state: Option<String>,
    pub(crate) code_challenge: String,
    pub(crate) resource: Option<String>,
    pub(crate) requested_scope: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthorizationCodeGrant {
    pub(crate) client_id: String,
    pub(crate) redirect_uri: String,
    pub(crate) code_challenge: String,
    pub(crate) resource: Option<String>,
    pub(crate) scope: String,
    pub(crate) backend_access_token: String,
}

#[derive(Debug, Clone)]
pub(crate) struct AccessTokenRecord {
    pub(crate) backend_access_token: String,
    pub(crate) gyazo_user: GyazoUserProfile,
}

#[derive(Debug, Clone)]
pub(crate) struct AuthorizedSession {
    pub(crate) record: AccessTokenRecord,
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

    pub(crate) fn has_saved_oauth_token(&self) -> Result<bool> {
        let guard = self
            .auth_state
            .lock()
            .map_err(|_| anyhow!("auth state lock is poisoned"))?;
        Ok(guard.has_saved_oauth_token())
    }

    pub(crate) fn has_backend_api_credential(&self) -> Result<bool> {
        Ok(self.has_saved_oauth_token()? || self.auth_config.has_personal_access_token())
    }

    pub(crate) fn resolve_backend_access_token(&self) -> Result<Option<String>> {
        let guard = self
            .auth_state
            .lock()
            .map_err(|_| anyhow!("auth state lock is poisoned"))?;

        if let Some(stored_token) = &guard.stored_token {
            return Ok(Some(stored_token.access_token.clone()));
        }

        Ok(self.auth_config.personal_access_token())
    }

    pub(crate) fn set_pending_direct_login_state(&self, state: String) -> Result<()> {
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        guard.pending_direct_login_state = Some(state);
        Ok(())
    }

    pub(crate) fn take_pending_direct_login_state(&self) -> Result<Option<String>> {
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        Ok(guard.pending_direct_login_state.take())
    }

    pub(crate) fn insert_pending_authorization(
        &self,
        state: String,
        request: PendingAuthorizationRequest,
    ) -> Result<()> {
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        guard.pending_authorizations.insert(state, request);
        Ok(())
    }

    pub(crate) fn take_pending_authorization(
        &self,
        state: &str,
    ) -> Result<Option<PendingAuthorizationRequest>> {
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        Ok(guard.pending_authorizations.remove(state))
    }

    pub(crate) fn has_pending_authorization(&self, state: &str) -> Result<bool> {
        let guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        Ok(guard.pending_authorizations.contains_key(state))
    }

    pub(crate) fn register_client(&self, client: RegisteredClient) -> Result<String> {
        let client_id = Uuid::new_v4().to_string();
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        guard.registered_clients.insert(client_id.clone(), client);
        Ok(client_id)
    }

    pub(crate) fn registered_client(&self, client_id: &str) -> Result<Option<RegisteredClient>> {
        let guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        Ok(guard.registered_clients.get(client_id).cloned())
    }

    pub(crate) fn issue_authorization_code(&self, grant: AuthorizationCodeGrant) -> Result<String> {
        let code = Uuid::new_v4().to_string();
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        guard.authorization_codes.insert(code.clone(), grant);
        Ok(code)
    }

    pub(crate) fn take_authorization_code(
        &self,
        code: &str,
    ) -> Result<Option<AuthorizationCodeGrant>> {
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        Ok(guard.authorization_codes.remove(code))
    }

    pub(crate) fn issue_access_token(&self, record: AccessTokenRecord) -> Result<String> {
        let token = Uuid::new_v4().to_string();
        let mut guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        guard.access_tokens.insert(token.clone(), record);
        Ok(token)
    }

    pub(crate) fn validate_access_token(&self, token: &str) -> Result<Option<AccessTokenRecord>> {
        let guard = self
            .oauth_session
            .lock()
            .map_err(|_| anyhow!("oauth session lock is poisoned"))?;
        Ok(guard.access_tokens.get(token).cloned())
    }

    pub(crate) fn authorized_session(&self, token: &str) -> Result<Option<AuthorizedSession>> {
        Ok(self
            .validate_access_token(token)?
            .map(|record| AuthorizedSession { record }))
    }
}
