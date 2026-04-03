# Changelog

## Unreleased

- `service` サブコマンド (`install` / `uninstall` / `status`) を追加しました。HTTP サーバーを OS サービスとして常駐させることができます。
  - Linux: systemd user service
  - macOS: launchd LaunchAgent
  - Windows: タスクスケジューラ (ログオン時に実行)
- `env init` 完了時にサービスが未登録の場合、`service install` のヒントを表示するようにしました。

## 0.3.0 - 2026-04-03

- `config` サブコマンド (`init` / `show` / `get` / `set` / `unset` / `path`) を追加しました。
- `env` サブコマンド (`init` / `show` / `get` / `set` / `unset` / `path`) を追加しました。
- `config init` で対話形式の初期設定を追加しました。OAuth アプリケーション登録のガイド、コールバック URL のクリップボードコピー、ブラウザでの登録ページ表示を含みます。
- `--config-dir` グローバルオプションと `config set config_dir` による設定ディレクトリの変更を追加しました。
- 設定ディレクトリのパスを `dirs` クレートに切り替え、Linux / macOS / Windows のクロスプラットフォームに対応しました。
- `env show` / `env get` でシークレット値を TTY ではマスク、パイプでは生値で出力するようにしました。
- `config` / `env` コマンドを `RuntimeConfig::load()` より前にディスパッチし、壊れた設定ファイルでも復旧可能にしました。

## 0.2.0 - 2026-04-03

- `stdio` transport を追加し、HTTP transport なしでも MCP client から利用できるようにしました。
- `gyazo-mcp-server stdio --auth` を追加し、MCP login 非対応 client 向けの OAuth 認証準備を可能にしました。
- 実行時設定を `config.toml`、認証情報を `.env` へ分離しました。
- `stdio --auth` の完了処理を修正し、callback 後に終了しない不具合を解消しました。
- README と運用ガイドに `stdio` 利用手順、PAT の制約、client 設定例を追記しました。
