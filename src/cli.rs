use clap::{Parser, Subcommand};

#[derive(Debug, Parser)]
#[command(
    name = "gyazo-mcp-server",
    version,
    about = "Gyazo 向けのローカル HTTP / stdio MCP サーバー"
)]
pub(crate) struct Cli {
    #[command(subcommand)]
    pub(crate) command: Option<Command>,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub(crate) enum Command {
    /// stdio transport で MCP server を起動します
    Stdio,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Command};

    #[test]
    fn parses_default_http_mode_without_subcommand() {
        let cli = Cli::parse_from(["gyazo-mcp-server"]);

        assert_eq!(cli.command, None);
    }

    #[test]
    fn parses_stdio_subcommand() {
        let cli = Cli::parse_from(["gyazo-mcp-server", "stdio"]);

        assert_eq!(cli.command, Some(Command::Stdio));
    }
}
