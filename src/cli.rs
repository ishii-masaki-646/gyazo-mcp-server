use clap::{Args, Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "gyazo-mcp-server",
    version,
    about = "Gyazo 向けのローカル HTTP / stdio MCP サーバー"
)]
pub(crate) struct Cli {
    /// 設定ディレクトリのパスを上書きします
    #[arg(long, global = true)]
    pub(crate) config_dir: Option<String>,

    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum Command {
    /// stdio transport で MCP server を起動します
    Stdio(StdioArgs),
    /// config.toml の設定を表示・変更します
    Config(ConfigArgs),
    /// .env の環境変数を表示・変更します
    Env(EnvArgs),
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct StdioArgs {
    /// stdio 起動前に one-shot の OAuth 認証を行い、token を保存して終了します
    #[arg(long)]
    pub(crate) auth: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ConfigArgs {
    #[command(subcommand)]
    pub(crate) command: ConfigCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum ConfigCommand {
    /// 対話形式で config.toml を初期設定します
    Init,
    /// 現在の設定を一覧表示します
    Show,
    /// 指定したキーの設定値を取得します
    Get(ConfigGetArgs),
    /// 設定値を変更します
    Set(ConfigSetArgs),
    /// 設定値を削除します
    Unset(ConfigUnsetArgs),
    /// config.toml のファイルパスを表示します
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ConfigUnsetArgs {
    /// 設定キー (config_dir, tcp_port, oauth_callback_path, rust_log)
    pub(crate) key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ConfigGetArgs {
    /// 設定キー (config_dir, tcp_port, oauth_callback_path, rust_log)
    pub(crate) key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct ConfigSetArgs {
    /// 設定キー (tcp_port, oauth_callback_path, rust_log)
    pub(crate) key: String,
    /// 設定値
    pub(crate) value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct EnvArgs {
    #[command(subcommand)]
    pub(crate) command: EnvCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum EnvCommand {
    /// 対話形式で .env を初期設定します
    Init,
    /// 現在の環境変数設定を一覧表示します
    Show,
    /// 指定したキーの環境変数を取得します
    Get(EnvGetArgs),
    /// 環境変数を設定します
    Set(EnvSetArgs),
    /// 環境変数を削除します
    Unset(EnvUnsetArgs),
    /// .env のファイルパスを表示します
    Path,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct EnvUnsetArgs {
    /// 環境変数名
    pub(crate) key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct EnvGetArgs {
    /// 環境変数名 (GYAZO_MCP_CONFIG_DIR, GYAZO_MCP_OAUTH_CLIENT_ID, GYAZO_MCP_OAUTH_CLIENT_SECRET, GYAZO_MCP_PERSONAL_ACCESS_TOKEN)
    pub(crate) key: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Args)]
pub(crate) struct EnvSetArgs {
    /// 環境変数名 (GYAZO_MCP_OAUTH_CLIENT_ID, GYAZO_MCP_OAUTH_CLIENT_SECRET, GYAZO_MCP_PERSONAL_ACCESS_TOKEN)
    pub(crate) key: String,
    /// 値
    pub(crate) value: String,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::*;

    #[test]
    fn parses_default_http_mode_without_subcommand() {
        let cli = Cli::parse_from(["gyazo-mcp-server"]);

        assert_eq!(cli.command, None);
        assert_eq!(cli.config_dir, None);
    }

    #[test]
    fn parses_global_config_dir_option() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "--config-dir", "/tmp/test", "config", "show"]);

        assert_eq!(cli.config_dir, Some("/tmp/test".to_string()));
        assert_eq!(
            cli.command,
            Some(Command::Config(ConfigArgs {
                command: ConfigCommand::Show,
            }))
        );
    }

    #[test]
    fn parses_stdio_subcommand() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "stdio"]);

        assert_eq!(cli.command, Some(Command::Stdio(StdioArgs { auth: false })));
    }

    #[test]
    fn parses_stdio_auth_flag() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "stdio", "--auth"]);

        assert_eq!(cli.command, Some(Command::Stdio(StdioArgs { auth: true })));
    }

    #[test]
    fn parses_config_init() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "config", "init"]);

        assert_eq!(
            cli.command,
            Some(Command::Config(ConfigArgs {
                command: ConfigCommand::Init,
            }))
        );
    }

    #[test]
    fn parses_config_show() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "config", "show"]);

        assert_eq!(
            cli.command,
            Some(Command::Config(ConfigArgs {
                command: ConfigCommand::Show,
            }))
        );
    }

    #[test]
    fn parses_config_get() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "config", "get", "tcp_port"]);

        assert_eq!(
            cli.command,
            Some(Command::Config(ConfigArgs {
                command: ConfigCommand::Get(ConfigGetArgs {
                    key: "tcp_port".to_string(),
                }),
            }))
        );
    }

    #[test]
    fn parses_config_set() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "config", "set", "tcp_port", "19000"]);

        assert_eq!(
            cli.command,
            Some(Command::Config(ConfigArgs {
                command: ConfigCommand::Set(ConfigSetArgs {
                    key: "tcp_port".to_string(),
                    value: "19000".to_string(),
                }),
            }))
        );
    }

    #[test]
    fn parses_config_unset() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "config", "unset", "tcp_port"]);

        assert_eq!(
            cli.command,
            Some(Command::Config(ConfigArgs {
                command: ConfigCommand::Unset(ConfigUnsetArgs {
                    key: "tcp_port".to_string(),
                }),
            }))
        );
    }

    #[test]
    fn parses_config_path() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "config", "path"]);

        assert_eq!(
            cli.command,
            Some(Command::Config(ConfigArgs {
                command: ConfigCommand::Path,
            }))
        );
    }

    #[test]
    fn parses_env_init() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "env", "init"]);

        assert_eq!(
            cli.command,
            Some(Command::Env(EnvArgs {
                command: EnvCommand::Init,
            }))
        );
    }

    #[test]
    fn parses_env_get() {
        let cli = Cli::parse_from([
            "gyazo-mcp-server",
            "env",
            "get",
            "GYAZO_MCP_OAUTH_CLIENT_ID",
        ]);

        assert_eq!(
            cli.command,
            Some(Command::Env(EnvArgs {
                command: EnvCommand::Get(EnvGetArgs {
                    key: "GYAZO_MCP_OAUTH_CLIENT_ID".to_string(),
                }),
            }))
        );
    }

    #[test]
    fn parses_env_show() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "env", "show"]);

        assert_eq!(
            cli.command,
            Some(Command::Env(EnvArgs {
                command: EnvCommand::Show,
            }))
        );
    }

    #[test]
    fn parses_env_set() {
        let cli = Cli::parse_from([
            "gyazo-mcp-server",
            "env",
            "set",
            "GYAZO_MCP_OAUTH_CLIENT_ID",
            "my-id",
        ]);

        assert_eq!(
            cli.command,
            Some(Command::Env(EnvArgs {
                command: EnvCommand::Set(EnvSetArgs {
                    key: "GYAZO_MCP_OAUTH_CLIENT_ID".to_string(),
                    value: "my-id".to_string(),
                }),
            }))
        );
    }

    #[test]
    fn parses_env_unset() {
        let cli = Cli::parse_from([
            "gyazo-mcp-server",
            "env",
            "unset",
            "GYAZO_MCP_PERSONAL_ACCESS_TOKEN",
        ]);

        assert_eq!(
            cli.command,
            Some(Command::Env(EnvArgs {
                command: EnvCommand::Unset(EnvUnsetArgs {
                    key: "GYAZO_MCP_PERSONAL_ACCESS_TOKEN".to_string(),
                }),
            }))
        );
    }

    #[test]
    fn parses_env_path() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "env", "path"]);

        assert_eq!(
            cli.command,
            Some(Command::Env(EnvArgs {
                command: EnvCommand::Path,
            }))
        );
    }
}
