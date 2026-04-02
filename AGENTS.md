# Repository Guidelines

## プロジェクト構成とモジュール整理
このリポジトリは小規模な Rust のバイナリクレートよ。`src/main.rs` は起動処理だけにとどめて、MCP サーバー本体は `src/server.rs`、ツール群は `src/tools/` 配下に分けてね。今後ツールを増やすときも、`src/tools/basic.rs` のように責務ごとにファイルを分ける前提で進めること。`Cargo.toml` はパッケージ定義と依存関係、`Cargo.lock` は依存バージョン固定、`config.toml` はローカル実行用の設定ファイル。ビルド成果物は `target/` に出るから、追跡対象にはしないこと。

## ビルド・テスト・開発コマンド
- `cargo run`: ローカルでビルドしてサーバーを起動する
- `cargo build`: 実際に起動可能なバイナリが生成できるか確認する
- `cargo check`: 実行ファイルを作らずに高速でコンパイル確認する
- `cargo test`: ユニットテストと統合テストを実行する
- `cargo fmt`: Rust 標準フォーマットを適用する
- `cargo clippy --all-targets --all-features`: レビュー前に lint を確認する

コードを更新したら、少なくとも `cargo build` でバイナリ生成が通ることを確認してね。プルリクエスト前は `cargo fmt`、`cargo build`、`cargo clippy --all-targets --all-features` を通すこと。

## コーディング規約と命名
Rust 2024 edition を前提に、整形は標準の `rustfmt` に従うこと。インデントは 4 スペース、関数は小さく保ち、読みやすさに効く場面では型を明示してね。命名は Rust の慣例に合わせること。
- モジュール名・ファイル名: `snake_case`
- 関数名・変数名: `snake_case`
- 型・トレイト・列挙型: `UpperCamelCase`
- 定数: `SCREAMING_SNAKE_CASE`

副作用はできるだけ境界に寄せて、再利用できる処理はテストしやすい関数やモジュールへ切り出してね。`#[allow(...)]` 系の抑制は原則禁止で、まず警告やエラーの原因をコード側で解消すること。どうしても必要な場合だけ、理由を明確にしたうえで最小範囲に限定して使ってね。

## テスト方針
まだテストはないけど、新しい振る舞いを追加するなら対応するテストも一緒に入れてね。ユニットテストは `#[cfg(test)]` で実装の近くに置き、公開挙動や CLI 風の流れを確認したい場合は `tests/` 配下に統合テストを追加するといいわ。テスト名は `returns_error_for_missing_token` みたいに、期待する挙動がそのまま分かる形にしてね。

## コミットとプルリクエスト
履歴はまだ少ないから、コミットメッセージは `Add Gyazo upload client` みたいな命令形で、短く明確に書くこと。件名は 72 文字前後までに収めて、1コミットごとにレビューしやすい単位へ分けてね。プルリクエストには変更概要、実行した確認コマンド（`cargo build`、`cargo test`、`cargo clippy` など）、設定やプロトコルに関する前提があればそれも書いておくこと。MCP の挙動を変える場合は、リクエストやレスポンスの例も添えると分かりやすいわ。

## 設定とセキュリティ
`config.toml` や `.env` に秘密情報、API トークン、端末固有の値をコミットしないこと。.env 系ファイルはローカル専用として扱って、必要なら `.env.example` だけを共有してね。`client_id`、`client_secret`、`PAT` のような秘密情報は `~/.config/gyazo-mcp-server/.env` に置いて、`GYAZO_MCP_TCP_PORT` や `GYAZO_MCP_OAUTH_CALLBACK_PATH` のような起動設定は MCP クライアント側の `mcp_servers.Gyazo.env` で渡す前提にすると分かりやすいわ。設定例が必要なら、実値ではなくプレースホルダーを使って、必要なキーは PR か README に書いて共有すること。
