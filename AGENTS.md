# Repository Guidelines

## プロジェクト構成とモジュール整理
このリポジトリは小規模な Rust のバイナリクレートです。`src/main.rs` は起動処理のみにとどめ、MCP サーバー本体は `src/server.rs`、ツール群は `src/tools/` 配下、設定管理は `src/runtime_config.rs` と `src/auth/`、OS サービス管理は `src/service.rs` に分けてください。今後ツールを増やす場合も、`src/tools/basic.rs` のように責務ごとにファイルを分ける前提で進めてください。`Cargo.toml` はパッケージ定義と依存関係、`Cargo.lock` は依存バージョン固定です。ビルド成果物は `target/` に出力されるため、追跡対象にはしないでください。

## ビルド・テスト・開発コマンド
- `cargo run`: ローカルでビルドしてサーバーを起動する
- `cargo build`: 実際に起動可能なバイナリが生成できるか確認する
- `cargo check`: 実行ファイルを作らずに高速でコンパイル確認する
- `cargo test`: ユニットテストと統合テストを実行する
- `cargo fmt`: Rust 標準フォーマットを適用する
- `cargo clippy --all-targets --all-features`: レビュー前に lint を確認する

コードを更新したら、少なくとも `cargo build` でバイナリ生成が通ることを確認してください。コミット前は GitHub Actions の CI と同じチェックを手元で通してください。

```bash
cargo test
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
```

`cargo fmt --check` で差分が出た場合は `cargo fmt` で整形してからコミットしてください。

## コーディング規約と命名
Rust 2024 edition を前提とし、整形は標準の `rustfmt` に従ってください。インデントは 4 スペースとし、関数は小さく保ち、読みやすさに有効な場面では型を明示してください。命名は Rust の慣例に合わせてください。
- モジュール名・ファイル名: `snake_case`
- 関数名・変数名: `snake_case`
- 型・トレイト・列挙型: `UpperCamelCase`
- 定数: `SCREAMING_SNAKE_CASE`

副作用はできるだけ境界に寄せ、再利用できる処理はテストしやすい関数やモジュールへ切り出してください。`#[allow(...)]` 系の抑制は原則禁止とし、まず警告やエラーの原因をコード側で解消してください。どうしても必要な場合だけ、理由を明確にしたうえで最小範囲に限定して使用してください。
アプリケーションメッセージ、CLI 出力、エラー文言、README などの公開向け文言は、会話口調ではなく公開向けの事務的な敬体で記述してください。

## テスト方針
新しい振る舞いを追加する場合は対応するテストも同時に追加してください。ユニットテストは `#[cfg(test)]` で実装の近くに置き、公開挙動や CLI 風の流れを確認したい場合は `tests/` 配下に統合テストを追加してください。テスト名は `returns_error_for_missing_token` のように、期待する挙動がそのまま分かる形にしてください。

レビューやテスト中に発見されたバグを修正する場合は、そのバグを再現する回帰テストも同時に追加してください。特に起動順序や設定ファイルの bootstrap に関する問題は、通常のユニットテストでは拾いにくいため、不変条件のアサーション（例: 特定のキーが許可リストに含まれないこと、派生パスが基底ディレクトリ配下であること）で防御してください。

現在のテストカバレッジの主な領域:
- CLI パース (`cli.rs`): 全サブコマンドとオプションの解析
- Gyazo API (`gyazo_api.rs`): 画像 ID 正規化、MIME 推定、リソース URI のラウンドトリップ
- 認証 (`auth/`): トークン保存・読み込み、HMAC 署名・検証、OAuth URL 構築、セッション永続化
- MCP OAuth (`mcp_oauth.rs`): PKCE 検証、リダイレクト URI 構築
- 設定管理 (`auth/config.rs`): .env パース、シークレットマスク、`GYAZO_MCP_CONFIG_DIR` の排除ガード
- パス解決 (`auth/paths.rs`): デフォルトパスのフォールバック、派生パスの整合性
- サービス管理 (`service.rs`): バイナリ検出、サービス定義の内容検証、`is_installed` の安全性
- Docker / コンテナ (`runtime_config.rs`): コンテナ検出、`bind_address` / `base_url` のフォールバック

## コミットとプルリクエスト
コミットメッセージは `Gyazo upload client を追加` のように日本語で、短く明確に記述してください。件名は 72 文字前後までに収め、1コミットごとにレビューしやすい単位へ分けてください。プルリクエストには変更概要、実行した確認コマンド（`cargo build`、`cargo test`、`cargo clippy` など）、設定やプロトコルに関する前提があればそれも記載してください。MCP の挙動を変える場合は、リクエストやレスポンスの例も添えると分かりやすくなります。

## 設定とセキュリティ
`config.toml` や `.env` に秘密情報、API トークン、端末固有の値をコミットしないでください。.env 系ファイルはローカル専用として扱い、必要な場合は `.env.example` のみを共有してください。設定ファイルはプラットフォームの標準ディレクトリ (`dirs::config_dir()` ベース) に配置されます。Linux は `~/.config/gyazo-mcp-server/`、macOS は `~/Library/Application Support/gyazo-mcp-server/`、Windows は `AppData\Roaming\gyazo-mcp-server\` です。動作設定は `config.toml`、認証情報は `.env` に分けて扱ってください。`tcp_port`、`oauth_callback_path`、`rust_log` は `config.toml` を正とし、一時上書きが必要な場合だけ `GYAZO_MCP_TCP_PORT`、`GYAZO_MCP_OAUTH_CALLBACK_PATH`、`RUST_LOG` を使ってください。`GYAZO_MCP_OAUTH_CLIENT_ID`、`GYAZO_MCP_OAUTH_CLIENT_SECRET`、`GYAZO_MCP_PERSONAL_ACCESS_TOKEN` は `.env` で管理してください。設定例が必要な場合は、実値ではなくプレースホルダーを用い、必要なキーは PR か README に記載して共有してください。

## 設定管理コマンド
- `config` サブコマンド (`init` / `show` / `get` / `set` / `unset` / `path`) は `config.toml` と `config_dir` を管理します。
- `env` サブコマンド (`init` / `show` / `get` / `set` / `unset` / `path`) は `.env` を管理します。
- `config_dir` の永続化は `config set config_dir` で行い、常にデフォルト位置の `.env` に書き出します。`env set GYAZO_MCP_CONFIG_DIR` は受け付けません。
- `config` / `env` コマンドは `RuntimeConfig::load()` より前にディスパッチされるため、`config.toml` が壊れていても `config set` で復旧できます。

## Docker 運用メモ
- `Dockerfile` はマルチステージビルド (`rust:1-bookworm` → `debian:bookworm-slim`)。glibc バージョンを揃えるためビルダーとランタイムを同じ Debian リリースにしてください。
- コンテナ内では `/.dockerenv` の存在検出により `bind_address` が自動的に `0.0.0.0` になります。
- `base_url` は `0.0.0.0` バインド時に `127.0.0.1` にフォールバックします。LAN 向けに公開する場合は `GYAZO_MCP_BASE_URL` で明示指定してください。
- `docker-compose.yml` のボリュームマウントはホスト OS によってパスが異なります（Linux/macOS: `${HOME}/.config/...`、Windows: `${APPDATA}/...`）。
- リリース時に GitHub Actions から ghcr.io と Docker Hub にマルチアーキイメージ (amd64 + arm64) が自動 push されます。
- `mcp-registry/` ディレクトリに Docker MCP Catalog への登録用メタデータ (`server.yaml` / `tools.json` / `readme.md`) を管理します。

## 配信チャネル
- **crates.io**: `cargo install gyazo-mcp-server` でインストール。`cargo publish` で公開。
- **Docker Hub / ghcr.io**: リリース CI でマルチアーキイメージ (amd64 + arm64) を自動 push。
- **Homebrew tap**: `ishii-masaki-646/homebrew-tap` リポジトリで formula を管理。リリース CI で formula のバージョンとチェックサムを自動更新。`HOMEBREW_TAP_TOKEN` (PAT) が必要。
- **winget**: `winget/` ディレクトリに manifest を管理。microsoft/winget-pkgs に PR で登録。
- **GitHub Releases**: タグ push 時にバイナリ (Linux / macOS / Windows) と CHANGELOG からのリリースノートを自動生成。
- **Docker MCP Catalog**: `mcp-registry/` のメタデータを docker/mcp-registry に PR で登録。

## Transport 運用メモ
- HTTP transport は `/mcp` endpoint を利用し、MCP login 対応 client からの利用を基本としてください。
- stdio transport は `gyazo-mcp-server stdio` で起動してください。
- MCP login 非対応 client 向けの認証準備は `gyazo-mcp-server stdio --auth` を案内してください。
- `stdio --auth` は一時的なローカル callback サーバーとブラウザ認可を必要とするため、その前提も README に明記してください。
- client 設定例を追加する場合は、少なくとも TOML 形式 1 例と JSON 形式 1 例を維持してください。

## サービス管理
- `service install` / `uninstall` / `status` は OS ごとのサービス登録を管理します。
- Linux は systemd user service、macOS は launchd LaunchAgent、Windows はタスクスケジューラを使用します。
- systemd の user manager が利用できない Linux や BSD 等は `systemctl --user daemon-reload` の失敗で検出し、手動案内にフォールバックします。
- `service` コマンドも `config` / `env` と同様に `RuntimeConfig::load()` より前に早期ディスパッチされます。
- `--config-dir` で一時 override した状態での `service install` は、永続化されていないか永続化値と不一致の場合に警告と確認を表示します。
- `is_installed()` は `env init` 完了時のヒント表示に使用されます。サービス登録済みの場合はヒントを表示しません。
- systemd unit ファイルのパスはダブルクォートで囲んでおり、空白を含むパスにも対応しています。
