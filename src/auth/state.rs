use std::path::PathBuf;

use anyhow::Result;

use crate::auth::{
    paths,
    token_store::{StoredToken, load_token},
};

#[derive(Debug, Clone, Default)]
pub(crate) struct AuthState {
    pub(crate) config_file_path: Option<PathBuf>,
    pub(crate) token_file_path: Option<PathBuf>,
    pub(crate) stored_token: Option<StoredToken>,
}

impl AuthState {
    pub(crate) fn load() -> Result<Self> {
        let config_file_path = paths::config_file_path();
        let token_file_path = paths::token_file_path();
        let stored_token = match token_file_path.as_deref() {
            Some(path) => load_token(path)?,
            None => None,
        };

        Ok(Self {
            config_file_path,
            token_file_path,
            stored_token,
        })
    }

    pub(crate) fn has_saved_oauth_token(&self) -> bool {
        self.stored_token.is_some()
    }
}
