# Changelog

## 0.5.0 - 2026-04-04

- Docker イメージを追加しました。コンテナ内では `/.dockerenv` 検出により自動的に `0.0.0.0` にバインドします。
- `bind_address` 設定を追加しました (`config.toml` / `GYAZO_MCP_BIND_ADDRESS`)。
- `base_url` 設定を追加しました (`config.toml` / `GYAZO_MCP_BASE_URL`)。OAuth metadata 等でクライアントに返す URL を明示指定できます。
- `docker-compose.yml` を追加しました。
- リリース CI で ghcr.io と Docker Hub にマルチアーキイメージ (amd64 + arm64) を自動 push するようにしました。
- CI に Docker ビルド・起動テストを追加しました。
- Docker MCP Catalog への登録用メタデータ (`mcp-registry/`) を追加しました。
- Homebrew tap (`brew tap ishii-masaki-646/tap`) を追加しました。macOS / Linux で `brew install gyazo-mcp-server` が利用可能です。
- リリース CI で Homebrew formula のバージョンとチェックサムを自動更新するようにしました。
- winget パッケージ manifest を追加しました。Windows で `winget install ishiimasaki646.gyazo-mcp-server` が利用可能です (microsoft/winget-pkgs への PR 承認後)。

## 0.4.0 - 2026-04-04

- `service` サブコマンド (`install` / `uninstall` / `status`) を追加しました。HTTP サーバーを OS サービスとして常駐させることができます。
  - Linux: systemd user service
  - macOS: launchd LaunchAgent
  - Windows: タスクスケジューラ (ログオン時に実行)
- `env init` 完了時にサービスが未登録の場合、`service install` のヒントを表示するようにしました。
- `--config-dir` が未永続化または永続化値と不一致の状態で `service install` を実行する場合、警告と確認を表示するようにしました。
- Linux の systemd user manager の検出を `systemctl --user daemon-reload` に改善し、user manager が動作していない環境での誤登録を防止しました。
- systemd unit ファイルのパスをダブルクォートで囲み、空白を含むパスに対応しました。

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
