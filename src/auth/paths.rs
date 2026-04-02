use std::{env, path::PathBuf};

pub(crate) fn config_dir() -> Option<PathBuf> {
    let home = env::var_os("HOME")?;
    Some(PathBuf::from(home).join(".config/gyazo-mcp-server"))
}

pub(crate) fn env_file_path() -> Option<PathBuf> {
    Some(config_dir()?.join(".env"))
}

pub(crate) fn config_file_path() -> Option<PathBuf> {
    Some(config_dir()?.join("config.toml"))
}

pub(crate) fn token_file_path() -> Option<PathBuf> {
    Some(config_dir()?.join("token.toml"))
}
