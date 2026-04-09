use std::{fs, io::IsTerminal, path::PathBuf, process::Command};

use anyhow::{Context, Result, bail};
use inquire::Confirm;

use crate::auth::{config as auth_config, paths};

const SERVICE_NAME: &str = "gyazo-mcp-server";

pub(crate) fn install() -> Result<()> {
    // --config-dir で一時 override されているが永続化されていない場合、
    // 常駐後のサービスはデフォルトの設定ディレクトリに戻ってしまう。
    // 意図しない設定不一致を防ぐため、確認を求める。
    if paths::has_config_dir_override() {
        let persisted = auth_config::read_config_dir_from_default_env();
        let override_dir = paths::config_dir()
            .map(|d| d.display().to_string())
            .unwrap_or_default();

        let mismatch = match &persisted {
            None => {
                // 永続化されていない
                Some(format!(
                    "警告: --config-dir が指定されていますが、永続化されていません。\n\
                     \x20 現在の override: {override_dir}\n\
                     \x20 常駐後のサービスはデフォルトの設定ディレクトリを使用します。\n\
                     \n\
                     永続化するには:\n\
                     \x20 gyazo-mcp-server config set config_dir {override_dir}"
                ))
            }
            Some(persisted_dir) if persisted_dir != &override_dir => {
                // 永続化されているが --config-dir と異なる
                Some(format!(
                    "警告: --config-dir と永続化された config_dir が異なります。\n\
                     \x20 --config-dir:  {override_dir}\n\
                     \x20 永続化済み:    {persisted_dir}\n\
                     \x20 常駐後のサービスは永続化された方 ({persisted_dir}) を使用します。\n\
                     \n\
                     --config-dir の値で上書きするには:\n\
                     \x20 gyazo-mcp-server config set config_dir {override_dir}"
                ))
            }
            _ => None, // 一致している
        };

        if let Some(message) = mismatch {
            eprintln!("{message}");
            eprintln!();

            if std::io::stdout().is_terminal() {
                let proceed = Confirm::new("このままサービスを登録しますか？")
                    .with_default(false)
                    .prompt()?;
                if !proceed {
                    println!("中断しました。");
                    return Ok(());
                }
            } else {
                bail!(
                    "--config-dir と永続化された config_dir が一致しない状態でのサービス登録は中断されました。"
                );
            }
        }
    }

    let binary = find_binary()?;

    #[cfg(target_os = "linux")]
    return install_systemd(&binary);

    #[cfg(target_os = "macos")]
    return install_launchd(&binary);

    #[cfg(target_os = "windows")]
    return install_windows_task(&binary);

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    {
        let _ = binary;
        bail!(
            "この OS ではサービスの自動登録に対応していません。手動でサービス設定を行ってください。"
        );
    }
}

pub(crate) fn uninstall() -> Result<()> {
    #[cfg(target_os = "linux")]
    return uninstall_systemd();

    #[cfg(target_os = "macos")]
    return uninstall_launchd();

    #[cfg(target_os = "windows")]
    return uninstall_windows_task();

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    bail!("この OS ではサービスの自動登録に対応していません。手動でサービス設定を行ってください。");
}

pub(crate) fn status() -> Result<()> {
    #[cfg(target_os = "linux")]
    return status_systemd();

    #[cfg(target_os = "macos")]
    return status_launchd();

    #[cfg(target_os = "windows")]
    return status_windows_task();

    #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
    bail!("この OS ではサービスの自動登録に対応していません。手動でサービス設定を行ってください。");
}

/// サービスが登録済みかどうかを返す。
/// 検出できない環境では false を返す。
pub(crate) fn is_installed() -> bool {
    is_installed_impl()
}

#[cfg(target_os = "linux")]
fn is_installed_impl() -> bool {
    systemd_unit_path().is_ok_and(|p| p.exists())
}

#[cfg(target_os = "macos")]
fn is_installed_impl() -> bool {
    launchd_plist_path().is_ok_and(|p| p.exists())
}

#[cfg(target_os = "windows")]
fn is_installed_impl() -> bool {
    Command::new("schtasks")
        .args(["/Query", "/TN", task_name()])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
fn is_installed_impl() -> bool {
    false
}

/// 現在実行中のバイナリのパスを取得する
fn find_binary() -> Result<PathBuf> {
    std::env::current_exe().context("実行中のバイナリのパスを取得できませんでした")
}

// ---------------------------------------------------------------------------
// Linux (systemd)
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
fn systemd_unit_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME 環境変数が設定されていません")?;
    Ok(PathBuf::from(home).join(".config/systemd/user"))
}

#[cfg(target_os = "linux")]
fn systemd_unit_path() -> Result<PathBuf> {
    Ok(systemd_unit_dir()?.join(format!("{SERVICE_NAME}.service")))
}

/// systemd の user manager が利用可能かどうかを確認する。
/// systemctl バイナリの存在だけでなく、user manager が動作しているかを
/// daemon-reload で確認する（unit file の変更なしでも安全に実行できるため）。
#[cfg(target_os = "linux")]
fn has_systemd_user_manager() -> bool {
    Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .output()
        .is_ok_and(|o| o.status.success())
}

#[cfg(target_os = "linux")]
fn generate_systemd_unit(binary: &std::path::Path) -> String {
    // systemd unit ではパス中の空白が分割されるため、ダブルクォートで囲む
    let env_file = paths::env_file_path()
        .map(|p| format!("EnvironmentFile=-\"{}\"", p.display()))
        .unwrap_or_default();

    format!(
        "[Unit]
Description=Gyazo MCP Server
After=network.target

[Service]
Type=simple
ExecStart=\"{binary}\"
Restart=on-failure
RestartSec=5
{env_file}

[Install]
WantedBy=default.target
",
        binary = binary.display(),
    )
}

#[cfg(target_os = "linux")]
fn install_systemd(binary: &std::path::Path) -> Result<()> {
    if !has_systemd_user_manager() {
        bail!(
            "systemd の user manager が利用できません。\n\
             systemctl --user daemon-reload が失敗しました。\n\
             この環境では手動でサービス設定を行ってください。"
        );
    }

    let unit_path = systemd_unit_path()?;
    let unit_content = generate_systemd_unit(binary);

    fs::create_dir_all(unit_path.parent().unwrap())?;
    fs::write(&unit_path, &unit_content).with_context(|| {
        format!(
            "ユニットファイルを書き込めませんでした: {}",
            unit_path.display()
        )
    })?;

    println!("ユニットファイルを作成しました: {}", unit_path.display());

    run_command("systemctl", &["--user", "daemon-reload"])?;
    run_command("systemctl", &["--user", "enable", SERVICE_NAME])?;
    run_command("systemctl", &["--user", "start", SERVICE_NAME])?;

    println!("\nサービスを登録・起動しました。");
    println!("  状態確認: gyazo-mcp-server service status");
    println!("  ログ確認: journalctl --user -u {SERVICE_NAME} -f");
    Ok(())
}

#[cfg(target_os = "linux")]
fn uninstall_systemd() -> Result<()> {
    if !has_systemd_user_manager() {
        bail!("systemd の user manager が利用できません。");
    }

    // 停止・無効化はエラーでも続行（既に停止済みの場合がある）
    let _ = run_command("systemctl", &["--user", "stop", SERVICE_NAME]);
    let _ = run_command("systemctl", &["--user", "disable", SERVICE_NAME]);

    let unit_path = systemd_unit_path()?;
    if unit_path.exists() {
        fs::remove_file(&unit_path).with_context(|| {
            format!(
                "ユニットファイルを削除できませんでした: {}",
                unit_path.display()
            )
        })?;
        println!("ユニットファイルを削除しました: {}", unit_path.display());
    }

    run_command("systemctl", &["--user", "daemon-reload"])?;

    println!("サービス登録を解除しました。");
    Ok(())
}

#[cfg(target_os = "linux")]
fn status_systemd() -> Result<()> {
    if !has_systemd_user_manager() {
        bail!("systemd の user manager が利用できません。");
    }

    let unit_path = systemd_unit_path()?;
    if !unit_path.exists() {
        println!("サービスは登録されていません。");
        println!("  登録: gyazo-mcp-server service install");
        return Ok(());
    }

    // systemctl status は非 active でも exit code 3 を返すので、出力だけ表示
    let output = Command::new("systemctl")
        .args(["--user", "status", SERVICE_NAME])
        .output()
        .context("systemctl status の実行に失敗しました")?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// macOS (launchd)
// ---------------------------------------------------------------------------

#[cfg(target_os = "macos")]
fn launchd_plist_dir() -> Result<PathBuf> {
    let home = std::env::var("HOME").context("HOME 環境変数が設定されていません")?;
    Ok(PathBuf::from(home).join("Library/LaunchAgents"))
}

#[cfg(target_os = "macos")]
fn launchd_label() -> String {
    "com.gyazo.mcp-server".to_string()
}

#[cfg(target_os = "macos")]
fn launchd_plist_path() -> Result<PathBuf> {
    Ok(launchd_plist_dir()?.join(format!("{}.plist", launchd_label())))
}

#[cfg(target_os = "macos")]
fn generate_launchd_plist(binary: &std::path::Path) -> String {
    let label = launchd_label();
    let log_dir = paths::config_dir()
        .map(|d| d.display().to_string())
        .unwrap_or_else(|| "/tmp".to_string());

    format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>{label}</string>
    <key>ProgramArguments</key>
    <array>
        <string>{binary}</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <dict>
        <key>SuccessfulExit</key>
        <false/>
    </dict>
    <key>StandardOutPath</key>
    <string>{log_dir}/stdout.log</string>
    <key>StandardErrorPath</key>
    <string>{log_dir}/stderr.log</string>
</dict>
</plist>
"#,
        binary = binary.display(),
    )
}

#[cfg(target_os = "macos")]
fn install_launchd(binary: &std::path::Path) -> Result<()> {
    let plist_path = launchd_plist_path()?;
    let plist_content = generate_launchd_plist(binary);

    fs::create_dir_all(plist_path.parent().unwrap())?;
    fs::write(&plist_path, &plist_content)
        .with_context(|| format!("plist を書き込めませんでした: {}", plist_path.display()))?;

    println!("plist を作成しました: {}", plist_path.display());

    run_command("launchctl", &["load", &plist_path.display().to_string()])?;

    println!("\nサービスを登録・起動しました。");
    println!("  状態確認: gyazo-mcp-server service status");
    Ok(())
}

#[cfg(target_os = "macos")]
fn uninstall_launchd() -> Result<()> {
    let plist_path = launchd_plist_path()?;

    if plist_path.exists() {
        let _ = run_command("launchctl", &["unload", &plist_path.display().to_string()]);
        fs::remove_file(&plist_path)
            .with_context(|| format!("plist を削除できませんでした: {}", plist_path.display()))?;
        println!("plist を削除しました: {}", plist_path.display());
    }

    println!("サービス登録を解除しました。");
    Ok(())
}

#[cfg(target_os = "macos")]
fn status_launchd() -> Result<()> {
    let plist_path = launchd_plist_path()?;
    if !plist_path.exists() {
        println!("サービスは登録されていません。");
        println!("  登録: gyazo-mcp-server service install");
        return Ok(());
    }

    let label = launchd_label();
    let output = Command::new("launchctl")
        .args(["list", &label])
        .output()
        .context("launchctl list の実行に失敗しました")?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("サービスは登録されていますが、現在実行されていません。");
        println!("  plist: {}", plist_path.display());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Windows (タスクスケジューラ)
// ---------------------------------------------------------------------------

#[cfg(target_os = "windows")]
fn task_name() -> &'static str {
    "GyazoMcpServer"
}

#[cfg(target_os = "windows")]
fn scripts_dir() -> Result<PathBuf> {
    paths::config_dir().ok_or_else(|| anyhow::anyhow!("設定ディレクトリを特定できませんでした"))
}

/// Windows PowerShell 5.x が生成する標準出力の文字コードを UTF-8 に固定する
/// プレリュード。Rust 側で `String::from_utf8_lossy` する前提なので、これを
/// 仕込まないと OEM コードページ (日本語環境では CP932) で書き出されて
/// 文字化けする。`.ps1` 自体は BOM 付き UTF-8 で書き出している。
#[cfg(target_os = "windows")]
fn ps1_utf8_prelude() -> &'static str {
    "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8\n\
     $OutputEncoding = [System.Text.Encoding]::UTF8\n"
}

#[cfg(target_os = "windows")]
fn generate_install_ps1(binary: &std::path::Path) -> String {
    // タスクスケジューラから直接 EXE を起動するとフォアグラウンドのコンソール
    // ウィンドウが残ってしまうため、`powershell.exe -WindowStyle Hidden` の中で
    // さらに `Start-Process -WindowStyle Hidden` を呼んでバックグラウンドに回す。
    // PowerShell の文字列リテラル中のダブルクォートは `""` でエスケープする。
    let task = task_name();
    let prelude = ps1_utf8_prelude();
    format!(
        r#"{prelude}$action = New-ScheduledTaskAction `
  -Execute "powershell.exe" `
  -Argument "-WindowStyle Hidden -Command ""Start-Process -WindowStyle Hidden -FilePath '{binary}'"""
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $env:USERNAME
$settings = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -ExecutionTimeLimit 0
Register-ScheduledTask -TaskName '{task}' -Action $action -Trigger $trigger -Settings $settings -Description 'Gyazo MCP Server (HTTP transport)'
Write-Host 'タスク "{task}" を登録しました。'
"#,
        binary = binary.display(),
    )
}

#[cfg(target_os = "windows")]
fn generate_uninstall_ps1() -> String {
    let task = task_name();
    let prelude = ps1_utf8_prelude();
    format!(
        r#"{prelude}Unregister-ScheduledTask -TaskName '{task}' -Confirm:$false
Write-Host 'タスク "{task}" を解除しました。'
"#,
    )
}

#[cfg(target_os = "windows")]
fn run_powershell_script(script_path: &std::path::Path) -> Result<()> {
    let output = Command::new("powershell")
        .args([
            "-ExecutionPolicy",
            "Bypass",
            "-File",
            &script_path.display().to_string(),
        ])
        .output()
        .with_context(|| {
            format!(
                "PowerShell スクリプトの実行に失敗しました: {}",
                script_path.display()
            )
        })?;

    print!("{}", String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        eprint!("{}", String::from_utf8_lossy(&output.stderr));
    }
    if !output.status.success() {
        bail!(
            "PowerShell スクリプトがエラーで終了しました (exit code: {:?})",
            output.status.code()
        );
    }
    Ok(())
}

/// PowerShell スクリプトを UTF-8 BOM 付きで書き出す。
///
/// Windows PowerShell 5.x は歴史的経緯により BOM なし UTF-8 を OS の現行
/// ANSI コードページとして解釈してしまい、日本語等の非 ASCII 文字が文字化け
/// したりパースエラーを起こす。BOM (`EF BB BF`) を先頭に付けると Unicode
/// として正しく扱われるため、`.ps1` を出力するときは必ずこのヘルパーを使う。
#[cfg(target_os = "windows")]
fn write_ps1_with_bom(path: &std::path::Path, content: &str) -> Result<()> {
    let mut bytes = Vec::with_capacity(content.len() + 3);
    bytes.extend_from_slice(b"\xEF\xBB\xBF");
    bytes.extend_from_slice(content.as_bytes());
    fs::write(path, bytes)
        .with_context(|| format!("スクリプトを書き込めませんでした: {}", path.display()))
}

#[cfg(target_os = "windows")]
fn install_windows_task(binary: &std::path::Path) -> Result<()> {
    let dir = scripts_dir()?;
    fs::create_dir_all(&dir)?;

    let script_path = dir.join("service-install.ps1");
    let script_content = generate_install_ps1(binary);
    write_ps1_with_bom(&script_path, &script_content)?;

    println!("スクリプトを作成しました: {}", script_path.display());

    run_powershell_script(&script_path)?;

    println!("\n  状態確認: gyazo-mcp-server service status");
    Ok(())
}

#[cfg(target_os = "windows")]
fn uninstall_windows_task() -> Result<()> {
    let dir = scripts_dir()?;
    fs::create_dir_all(&dir)?;

    let script_path = dir.join("service-uninstall.ps1");
    let script_content = generate_uninstall_ps1();
    write_ps1_with_bom(&script_path, &script_content)?;

    run_powershell_script(&script_path)?;

    // スクリプト自体も掃除
    let _ = fs::remove_file(dir.join("service-install.ps1"));
    let _ = fs::remove_file(&script_path);

    Ok(())
}

#[cfg(target_os = "windows")]
fn status_windows_task() -> Result<()> {
    // schtasks.exe の出力は OEM コードページ (日本語環境では CP932) なので、
    // Rust 側で UTF-8 として読むと文字化けする。PowerShell 経由で
    // `[Console]::OutputEncoding` を UTF-8 に固定してから schtasks を起動し、
    // 標準出力を UTF-8 で受け取る。
    let task = task_name();
    let command = format!(
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         schtasks.exe /Query /TN '{task}' /FO LIST /V"
    );
    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &command])
        .output()
        .context("schtasks の実行に失敗しました")?;

    if output.status.success() {
        print!("{}", String::from_utf8_lossy(&output.stdout));
    } else {
        println!("サービスは登録されていません。");
        println!("  登録: gyazo-mcp-server service install");
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// 共通ユーティリティ
// ---------------------------------------------------------------------------

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn run_command(program: &str, args: &[&str]) -> Result<()> {
    let output = Command::new(program)
        .args(args)
        .output()
        .with_context(|| format!("{program} の実行に失敗しました"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("{program} がエラーで終了しました: {stderr}");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_binary_returns_current_exe() {
        let binary = find_binary().unwrap();
        assert!(binary.exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_unit_contains_binary_path() {
        let binary = PathBuf::from("/usr/local/bin/gyazo-mcp-server");
        let unit = generate_systemd_unit(&binary);

        assert!(unit.contains("ExecStart=\"/usr/local/bin/gyazo-mcp-server\""));
        assert!(unit.contains("[Install]"));
        assert!(unit.contains("WantedBy=default.target"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_unit_quotes_paths_with_spaces() {
        let binary = PathBuf::from("/opt/my programs/gyazo-mcp-server");
        let unit = generate_systemd_unit(&binary);

        // 空白入りパスがダブルクォートで囲まれていること
        assert!(unit.contains("ExecStart=\"/opt/my programs/gyazo-mcp-server\""));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchd_plist_contains_binary_path() {
        let binary = PathBuf::from("/usr/local/bin/gyazo-mcp-server");
        let plist = generate_launchd_plist(&binary);

        assert!(plist.contains("/usr/local/bin/gyazo-mcp-server"));
        assert!(plist.contains("RunAtLoad"));
        assert!(plist.contains(&launchd_label()));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn install_ps1_contains_task_name() {
        let binary = PathBuf::from(r"C:\Users\test\.cargo\bin\gyazo-mcp-server.exe");
        let ps1 = generate_install_ps1(&binary);

        assert!(ps1.contains(task_name()));
        assert!(ps1.contains(r"C:\Users\test\.cargo\bin\gyazo-mcp-server.exe"));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn install_ps1_runs_binary_via_hidden_powershell() {
        // タスクスケジューラから直接 EXE を起動するとフォアグラウンドの
        // コンソールウィンドウが残るため、powershell.exe + Start-Process で
        // バックグラウンド化していることを保証する。
        let binary = PathBuf::from(r"C:\bin\gyazo-mcp-server.exe");
        let ps1 = generate_install_ps1(&binary);

        assert!(
            ps1.contains(r#"-Execute "powershell.exe""#),
            "powershell.exe を Execute に指定していません"
        );
        assert!(
            ps1.contains("Start-Process -WindowStyle Hidden"),
            "Start-Process でバックグラウンド起動にしていません"
        );
        assert!(
            ps1.contains("-WindowStyle Hidden -Command"),
            "powershell.exe の WindowStyle が Hidden ではありません"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn uninstall_ps1_contains_task_name() {
        let ps1 = generate_uninstall_ps1();
        assert!(ps1.contains(task_name()));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn generated_ps1_sets_utf8_output_encoding() {
        // Windows PowerShell 5.x の標準出力は既定で OEM コードページなので、
        // Rust 側で UTF-8 として読むと文字化けする。生成スクリプトの先頭で
        // `[Console]::OutputEncoding` を UTF-8 に固定していることを保証する。
        let install = generate_install_ps1(&PathBuf::from(r"C:\bin\gyazo-mcp-server.exe"));
        let uninstall = generate_uninstall_ps1();
        for (label, ps1) in [("install", &install), ("uninstall", &uninstall)] {
            assert!(
                ps1.contains("[Console]::OutputEncoding = [System.Text.Encoding]::UTF8"),
                "{label} ps1 に Console::OutputEncoding の UTF-8 化が含まれていません"
            );
            assert!(
                ps1.contains("$OutputEncoding = [System.Text.Encoding]::UTF8"),
                "{label} ps1 に $OutputEncoding の UTF-8 化が含まれていません"
            );
        }
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn write_ps1_with_bom_prepends_utf8_bom() {
        // Windows PowerShell 5.x が日本語を文字化けせず読めるよう、
        // 出力先頭に UTF-8 BOM (EF BB BF) が付くことを保証する。
        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("gyazo-mcp-ps1-bom-test-{unique}"));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("test.ps1");

        write_ps1_with_bom(&path, "Write-Host 'こんにちは'\n").unwrap();

        let bytes = std::fs::read(&path).unwrap();
        assert_eq!(&bytes[..3], b"\xEF\xBB\xBF", "UTF-8 BOM が付いていません");
        assert_eq!(
            std::str::from_utf8(&bytes[3..]).unwrap(),
            "Write-Host 'こんにちは'\n"
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_dir(&dir);
    }

    #[test]
    fn is_installed_returns_bool_without_panic() {
        // is_installed() がパニックせずに bool を返すことを確認。
        // 開発環境では登録済みの場合もあるため、値自体はアサートしない。
        let _result: bool = is_installed();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_unit_path_is_under_user_config() {
        let path = systemd_unit_path().unwrap();
        let path_str = path.display().to_string();
        assert!(path_str.contains(".config/systemd/user"));
        assert!(path_str.ends_with("gyazo-mcp-server.service"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_unit_includes_restart_on_failure() {
        let binary = PathBuf::from("/usr/bin/gyazo-mcp-server");
        let unit = generate_systemd_unit(&binary);
        assert!(unit.contains("Restart=on-failure"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchd_plist_path_is_under_launch_agents() {
        let path = launchd_plist_path().unwrap();
        let path_str = path.display().to_string();
        assert!(path_str.contains("Library/LaunchAgents"));
        assert!(path_str.ends_with(".plist"));
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchd_plist_includes_keep_alive() {
        let binary = PathBuf::from("/usr/local/bin/gyazo-mcp-server");
        let plist = generate_launchd_plist(&binary);
        assert!(plist.contains("KeepAlive"));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn has_systemd_user_manager_returns_bool_without_panic() {
        // systemd がある環境でもない環境でもパニックせず bool を返すことを確認
        let _result: bool = has_systemd_user_manager();
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn systemd_unit_does_not_contain_config_dir_flag() {
        // サービス定義に --config-dir を含めない（起動時の bootstrap で解決するため）
        let binary = PathBuf::from("/usr/bin/gyazo-mcp-server");
        let unit = generate_systemd_unit(&binary);
        assert!(
            !unit.contains("--config-dir"),
            "サービス定義に --config-dir を含めてはならない"
        );
    }

    #[cfg(target_os = "macos")]
    #[test]
    fn launchd_plist_does_not_contain_config_dir_flag() {
        let binary = PathBuf::from("/usr/local/bin/gyazo-mcp-server");
        let plist = generate_launchd_plist(&binary);
        assert!(
            !plist.contains("--config-dir"),
            "サービス定義に --config-dir を含めてはならない"
        );
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn install_ps1_does_not_contain_config_dir_flag() {
        let binary = PathBuf::from(r"C:\Users\test\.cargo\bin\gyazo-mcp-server.exe");
        let ps1 = generate_install_ps1(&binary);
        assert!(
            !ps1.contains("--config-dir"),
            "サービス定義に --config-dir を含めてはならない"
        );
    }
}
