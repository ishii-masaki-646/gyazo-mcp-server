use std::{
    fs,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};

use anyhow::{Context, Result, bail};
use inquire::{Confirm, Text};
use serde::Deserialize;
use tracing_subscriber::EnvFilter;
use unicode_width::UnicodeWidthStr;

use crate::auth::paths;

#[derive(Debug, Clone, Deserialize, Default)]
struct RuntimeConfigFile {
    tcp_port: Option<u16>,
    oauth_callback_path: Option<String>,
    rust_log: Option<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct RuntimeConfig {
    tcp_port: u16,
    oauth_callback_path: String,
    rust_log: Option<String>,
}

impl RuntimeConfig {
    pub(crate) fn load() -> Result<Self> {
        let file_config = load_runtime_config_file()?;

        let tcp_port = std::env::var("GYAZO_MCP_TCP_PORT")
            .ok()
            .map(|value| value.parse::<u16>())
            .transpose()?
            .or(file_config.tcp_port)
            .unwrap_or(18449);
        let oauth_callback_path = std::env::var("GYAZO_MCP_OAUTH_CALLBACK_PATH")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or(file_config.oauth_callback_path)
            .unwrap_or_else(|| "/oauth/callback".to_string());
        let rust_log = std::env::var("RUST_LOG")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or(file_config.rust_log);

        if !oauth_callback_path.starts_with('/') {
            bail!("GYAZO_MCP_OAUTH_CALLBACK_PATH must start with '/'");
        }
        if let Some(rust_log) = &rust_log {
            EnvFilter::try_new(rust_log).with_context(|| {
                format!("RUST_LOG / rust_log を解釈できませんでした: {rust_log}")
            })?;
        }

        Ok(Self {
            tcp_port,
            oauth_callback_path,
            rust_log,
        })
    }

    pub(crate) fn tracing_env_filter(&self) -> EnvFilter {
        self.rust_log
            .as_deref()
            .map(EnvFilter::new)
            .unwrap_or_else(|| EnvFilter::new("gyazo_mcp_server=info,rmcp=info"))
    }

    pub(crate) fn bind_address(&self) -> SocketAddr {
        SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), self.tcp_port)
    }

    pub(crate) fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}", self.tcp_port)
    }

    pub(crate) fn mcp_path(&self) -> &'static str {
        "/mcp"
    }

    pub(crate) fn protected_resource_metadata_root_path(&self) -> &'static str {
        "/.well-known/oauth-protected-resource"
    }

    pub(crate) fn protected_resource_metadata_path(&self) -> String {
        format!(
            "{}/{}",
            self.protected_resource_metadata_root_path(),
            self.mcp_path().trim_start_matches('/')
        )
    }

    pub(crate) fn authorization_server_metadata_path(&self) -> &'static str {
        "/.well-known/oauth-authorization-server"
    }

    pub(crate) fn authorization_endpoint_path(&self) -> &'static str {
        "/authorize"
    }

    pub(crate) fn token_endpoint_path(&self) -> &'static str {
        "/token"
    }

    pub(crate) fn registration_endpoint_path(&self) -> &'static str {
        "/register"
    }

    pub(crate) fn oauth_start_path(&self) -> &'static str {
        "/oauth/start"
    }

    pub(crate) fn oauth_callback_path(&self) -> &str {
        &self.oauth_callback_path
    }

    pub(crate) fn mcp_url(&self) -> String {
        format!("{}{}", self.base_url(), self.mcp_path())
    }

    pub(crate) fn protected_resource_metadata_url(&self) -> String {
        format!(
            "{}{}",
            self.base_url(),
            self.protected_resource_metadata_path()
        )
    }

    pub(crate) fn authorization_server_issuer(&self) -> String {
        self.base_url()
    }

    pub(crate) fn authorization_server_metadata_url(&self) -> String {
        format!(
            "{}{}",
            self.base_url(),
            self.authorization_server_metadata_path()
        )
    }

    pub(crate) fn authorization_endpoint_url(&self) -> String {
        format!("{}{}", self.base_url(), self.authorization_endpoint_path())
    }

    pub(crate) fn token_endpoint_url(&self) -> String {
        format!("{}{}", self.base_url(), self.token_endpoint_path())
    }

    pub(crate) fn registration_endpoint_url(&self) -> String {
        format!("{}{}", self.base_url(), self.registration_endpoint_path())
    }

    pub(crate) fn oauth_start_url(&self) -> String {
        format!("{}{}", self.base_url(), self.oauth_start_path())
    }

    pub(crate) fn oauth_callback_url(&self) -> String {
        format!("{}{}", self.base_url(), self.oauth_callback_path())
    }
}

fn load_runtime_config_file() -> Result<RuntimeConfigFile> {
    let Some(path) = paths::config_file_path() else {
        return Ok(RuntimeConfigFile::default());
    };

    if !path.exists() {
        return Ok(RuntimeConfigFile::default());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("config.toml を読み取れませんでした: {}", path.display()))?;
    toml::from_str(&contents)
        .with_context(|| format!("config.toml を解析できませんでした: {}", path.display()))
}

fn print_box(title: &str, lines: &[&str]) {
    // テキストの表示幅は width_cjk() で計算（※ 等の ambiguous 文字を幅2で扱う）。
    // 罫線文字（─│┌┐└┘）は多くの端末で幅1なので repeat 数はそのまま。

    // 1. 内容行の最大表示幅からボックス内側幅を確定
    let max_line_w = lines.iter().map(|l| l.width_cjk()).max().unwrap_or(0);
    let inner = max_line_w + 2;

    // 2. 内容行を組み立て（スペースで右パディング）
    let content_lines: Vec<String> = lines
        .iter()
        .map(|line| {
            let pad = inner - line.width_cjk();
            format!("│{line}{}│", " ".repeat(pad))
        })
        .collect();

    // 3. 罫線行を組み立て
    let title_w = title.width_cjk();
    let top = if title_w + 2 <= inner {
        let right = inner - 1 - title_w;
        format!("┌─{title}{}┐", "─".repeat(right))
    } else {
        format!("┌{}┐", "─".repeat(inner))
    };
    let bottom = format!("└{}┘", "─".repeat(inner));

    // 4. 出力
    println!("{top}");
    for line in &content_lines {
        println!("{line}");
    }
    println!("{bottom}");
}

fn prompt_with_default(key: &str, default: &str, description: &str) -> Result<String> {
    let input = Text::new(&format!("{key} [{default}]:"))
        .with_help_message(&format!("{description} 空 Enter で既定値を使用。"))
        .prompt()?;
    Ok(if input.is_empty() {
        default.to_string()
    } else {
        input
    })
}

const VALID_CONFIG_KEYS: &[&str] = &["config_dir", "tcp_port", "oauth_callback_path", "rust_log"];

pub(crate) fn show_config() -> Result<()> {
    let file_config = load_runtime_config_file()?;

    // config_dir は デフォルト位置の .env から読み取る
    let config_dir_from_env = read_config_dir_from_default_env();
    let config_dir_resolved = paths::config_dir()
        .map(|d| d.display().to_string())
        .unwrap_or_else(|| "(unknown)".to_string());
    if config_dir_from_env.is_some() {
        println!("config_dir = \"{config_dir_resolved}\" (.env)");
    } else {
        println!("config_dir = \"{config_dir_resolved}\" (default)");
    }

    let entries = [
        (
            "tcp_port",
            file_config.tcp_port.map(|v| v.to_string()),
            std::env::var("GYAZO_MCP_TCP_PORT").ok(),
            "18449".to_string(),
        ),
        (
            "oauth_callback_path",
            file_config.oauth_callback_path.clone(),
            std::env::var("GYAZO_MCP_OAUTH_CALLBACK_PATH")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            "/oauth/callback".to_string(),
        ),
        (
            "rust_log",
            file_config.rust_log.clone(),
            std::env::var("RUST_LOG")
                .ok()
                .filter(|v| !v.trim().is_empty()),
            "gyazo_mcp_server=info,rmcp=info".to_string(),
        ),
    ];

    for (key, file_value, env_value, default) in &entries {
        if let Some(value) = env_value {
            println!("{key} = \"{value}\" (env)");
        } else if let Some(value) = file_value {
            println!("{key} = \"{value}\" (config.toml)");
        } else {
            println!("{key} = \"{default}\" (default)");
        }
    }

    Ok(())
}

pub(crate) fn get_config(key: &str) -> Result<()> {
    if !VALID_CONFIG_KEYS.contains(&key) {
        bail!(
            "不明な設定キー: {key}\n有効なキー: {}",
            VALID_CONFIG_KEYS.join(", ")
        );
    }

    if key == "config_dir" {
        let value = paths::config_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_else(|| "(unknown)".to_string());
        println!("{value}");
        return Ok(());
    }

    let file_config = load_runtime_config_file()?;
    let value = match key {
        "tcp_port" => std::env::var("GYAZO_MCP_TCP_PORT")
            .ok()
            .or_else(|| file_config.tcp_port.map(|v| v.to_string()))
            .unwrap_or_else(|| "18449".to_string()),
        "oauth_callback_path" => std::env::var("GYAZO_MCP_OAUTH_CALLBACK_PATH")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or(file_config.oauth_callback_path)
            .unwrap_or_else(|| "/oauth/callback".to_string()),
        "rust_log" => std::env::var("RUST_LOG")
            .ok()
            .filter(|v| !v.trim().is_empty())
            .or(file_config.rust_log)
            .unwrap_or_else(|| "gyazo_mcp_server=info,rmcp=info".to_string()),
        _ => unreachable!(),
    };
    println!("{value}");
    Ok(())
}

pub(crate) fn init_config() -> Result<()> {
    let file_config = load_runtime_config_file()?;

    let current_config_dir = paths::config_dir()
        .map(|d| d.display().to_string())
        .unwrap_or_default();
    let current_port = file_config
        .tcp_port
        .map(|v| v.to_string())
        .unwrap_or_else(|| "18449".to_string());
    let current_callback = file_config
        .oauth_callback_path
        .unwrap_or_else(|| "/oauth/callback".to_string());
    let current_log = file_config
        .rust_log
        .unwrap_or_else(|| "gyazo_mcp_server=info,rmcp=info".to_string());

    println!();
    print_box(
        " 最初のステップ ",
        &[
            "  MCP サーバーの基本設定を行います。",
            "  tcp_port は他のアプリケーションと被らないよう注意してください。",
            "  それ以外は通常の場合、既定値のまま Enter で進めて問題ありません。",
        ],
    );
    println!();

    let config_dir = prompt_with_default(
        "config_dir",
        &current_config_dir,
        "設定ファイルや認証トークンを保存するディレクトリ。変更するとデフォルト位置の .env に保存されます。",
    )?;
    let port = prompt_with_default(
        "tcp_port",
        &current_port,
        "HTTP モードで MCP サーバーが待ち受ける TCP ポート番号。",
    )?;
    let callback = prompt_with_default(
        "oauth_callback_path",
        &current_callback,
        "Gyazo OAuth 認証のコールバックを受け取るパス。'/' で始まる必要があります。",
    )?;
    let log = prompt_with_default(
        "rust_log",
        &current_log,
        "tracing クレートのログフィルター (例: gyazo_mcp_server=debug,rmcp=info)。",
    )?;

    // 既定値と異なる場合のみ書き込む
    // config_dir は .env に書くので、変更時は先に保存して以降の書き込み先を切り替える
    if config_dir != current_config_dir {
        set_config("config_dir", &config_dir)?;
    }
    if port != current_port {
        set_config("tcp_port", &port)?;
    }
    if callback != current_callback {
        set_config("oauth_callback_path", &callback)?;
    }
    if log != current_log {
        set_config("rust_log", &log)?;
    }

    let callback_url = format!("http://127.0.0.1:{port}{callback}");
    println!("\n設定の初期化が完了しました");
    println!();
    println!("Gyazo OAuth アプリケーションの登録時に、以下のコールバック URL を設定してください:");
    println!("  {callback_url}");
    println!();
    let gyazo_new_app_url = "https://gyazo.com/oauth/applications/new";

    let step_line1 = "  Gyazo で OAuth アプリケーションを登録します。";
    let step_line2 = "  * 先に https://gyazo.com にログインしておく必要があります。";
    let step_line3 = "    未ログインのまま登録ページを開くと、ログイン後にキャプチャ一覧へ";
    let step_line4 = "    転送されることがあります。その場合は以下の URL を直接開いてください:";
    let step_line5 = format!("    {gyazo_new_app_url}");
    println!();
    print_box(
        " 次のステップ ",
        &[step_line1, step_line2, step_line3, step_line4, &step_line5],
    );
    println!();
    Text::new("Gyazo にログインできたら Enter を押してください。")
        .with_help_message("ブラウザで https://gyazo.com にログインしてから続行してください。")
        .prompt()?;

    let name_line = "  Name ............. 好きな名前でOK (おすすめ: \"Gyazo MCP Server\")";
    let url_line = format!("  Callback URL ..... {callback_url}");
    println!();
    print_box(
        " 登録フォームの入力について ",
        &[name_line, &url_line],
    );
    println!();

    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let copy = Confirm::new("コールバック URL をクリップボードにコピーしますか？")
            .with_default(true)
            .prompt()?;
        if copy {
            clipboard
                .set_text(&callback_url)
                .map_err(|e| anyhow::anyhow!("クリップボードへのコピーに失敗しました: {e}"))?;
            // Linux ではクリップボードマネージャーが内容を取得する前に
            // Clipboard がドロップされると中身が消えるため、少し待つ
            std::thread::sleep(std::time::Duration::from_millis(100));
            println!("コピーしました！");
        }
    }

    let open_browser =
        Confirm::new("OAuth アプリケーション登録ページをブラウザで開きますか？")
            .with_default(true)
            .prompt()?;
    if open_browser {
        open::that(gyazo_new_app_url)
            .map_err(|e| anyhow::anyhow!("ブラウザを開けませんでした: {e}"))?;
    }

    println!();
    print_box(
        " 登録完了後 ",
        &[
            "  1. Submit するとアプリケーション一覧に遷移します",
            "  2. 登録したアプリ名をクリックして詳細を開きます",
            "  3. 表示された Client ID と Client Secret を控えてください",
            "  4. 次のコマンドで設定します:",
            "",
            "    gyazo-mcp-server env init",
        ],
    );

    Ok(())
}

pub(crate) fn unset_config(key: &str) -> Result<()> {
    if !VALID_CONFIG_KEYS.contains(&key) {
        bail!(
            "不明な設定キー: {key}\n有効なキー: {}",
            VALID_CONFIG_KEYS.join(", ")
        );
    }

    // config_dir は常にデフォルト位置の .env から削除する
    if key == "config_dir" {
        return unset_config_dir_in_default_env();
    }

    let path = paths::config_file_path()
        .ok_or_else(|| anyhow::anyhow!("設定ディレクトリを特定できませんでした"))?;

    if !path.exists() {
        println!("{key} は設定されていません");
        return Ok(());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!("config.toml を読み取れませんでした: {}", path.display()))?;
    let mut table: toml::Table = toml::from_str(&contents)
        .with_context(|| format!("config.toml を解析できませんでした: {}", path.display()))?;

    if table.remove(key).is_none() {
        println!("{key} は設定されていません");
        return Ok(());
    }

    let raw = toml::to_string(&table).context("config.toml をシリアライズできませんでした")?;
    fs::write(&path, raw)
        .with_context(|| format!("config.toml に書き込めませんでした: {}", path.display()))?;

    println!("{key} を config.toml から削除しました");
    Ok(())
}

fn read_config_dir_from_default_env() -> Option<String> {
    let path = crate::auth::config::default_env_file_path()?;
    let contents = fs::read_to_string(path).ok()?;
    contents
        .lines()
        .find(|line| line.trim_start().starts_with("GYAZO_MCP_CONFIG_DIR="))
        .and_then(|line| line.split_once('='))
        .map(|(_, v)| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn set_config_dir_in_default_env(value: &str) -> Result<()> {
    let path = crate::auth::config::default_env_file_path()
        .ok_or_else(|| anyhow::anyhow!("デフォルトの設定ディレクトリを特定できませんでした"))?;

    let contents = if path.exists() {
        fs::read_to_string(&path)
            .with_context(|| format!(".env を読み取れませんでした: {}", path.display()))?
    } else {
        String::new()
    };

    let target = "GYAZO_MCP_CONFIG_DIR=";
    let mut found = false;
    let mut lines: Vec<String> = contents
        .lines()
        .map(|line: &str| {
            if line.trim_start().starts_with(target) {
                found = true;
                format!("GYAZO_MCP_CONFIG_DIR={value}")
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        lines.push(format!("GYAZO_MCP_CONFIG_DIR={value}"));
    }

    let mut output = lines.join("\n");
    if !output.ends_with('\n') {
        output.push('\n');
    }

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, output)
        .with_context(|| format!(".env に書き込めませんでした: {}", path.display()))?;

    println!("GYAZO_MCP_CONFIG_DIR = \"{value}\" をデフォルトの .env に保存しました");
    println!("  (保存先: {})", path.display());
    Ok(())
}

fn unset_config_dir_in_default_env() -> Result<()> {
    let path = crate::auth::config::default_env_file_path()
        .ok_or_else(|| anyhow::anyhow!("デフォルトの設定ディレクトリを特定できませんでした"))?;

    if !path.exists() {
        println!("config_dir は設定されていません");
        return Ok(());
    }

    let contents = fs::read_to_string(&path)
        .with_context(|| format!(".env を読み取れませんでした: {}", path.display()))?;

    let target = "GYAZO_MCP_CONFIG_DIR=";
    let original_len = contents.lines().count();
    let lines: Vec<&str> = contents
        .lines()
        .filter(|line| !line.trim_start().starts_with(target))
        .collect();

    if lines.len() == original_len {
        println!("config_dir は設定されていません");
        return Ok(());
    }

    let mut output = lines.join("\n");
    if !output.is_empty() && !output.ends_with('\n') {
        output.push('\n');
    }
    fs::write(&path, output)
        .with_context(|| format!(".env に書き込めませんでした: {}", path.display()))?;

    println!("config_dir をデフォルトの .env から削除しました");
    Ok(())
}

pub(crate) fn set_config(key: &str, value: &str) -> Result<()> {
    if !VALID_CONFIG_KEYS.contains(&key) {
        bail!(
            "不明な設定キー: {key}\n有効なキー: {}",
            VALID_CONFIG_KEYS.join(", ")
        );
    }

    // config_dir は常にデフォルト位置の .env に保存する。
    // 変更先の .env に書くと次回起動時にたどり着けなくなるため。
    if key == "config_dir" {
        return set_config_dir_in_default_env(value);
    }

    let path = paths::config_file_path()
        .ok_or_else(|| anyhow::anyhow!("設定ディレクトリを特定できませんでした"))?;

    let mut file_config = if path.exists() {
        let contents = fs::read_to_string(&path)
            .with_context(|| format!("config.toml を読み取れませんでした: {}", path.display()))?;
        toml::from_str::<toml::Table>(&contents)
            .with_context(|| format!("config.toml を解析できませんでした: {}", path.display()))?
    } else {
        toml::Table::new()
    };

    let toml_value = match key {
        "tcp_port" => {
            let port: u16 = value
                .parse()
                .with_context(|| format!("tcp_port は 0-65535 の整数で指定してください: {value}"))?;
            toml::Value::Integer(port.into())
        }
        _ => toml::Value::String(value.to_string()),
    };
    file_config.insert(key.to_string(), toml_value);

    let raw = toml::to_string(&file_config).context("config.toml をシリアライズできませんでした")?;

    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| {
            format!(
                "設定ディレクトリを作成できませんでした: {}",
                parent.display()
            )
        })?;
    }

    fs::write(&path, raw)
        .with_context(|| format!("config.toml に書き込めませんでした: {}", path.display()))?;

    println!("{key} = \"{value}\" を config.toml に保存しました");
    Ok(())
}
