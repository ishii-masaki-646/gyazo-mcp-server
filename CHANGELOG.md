# Changelog

## 0.2.0 - 2026-04-03

- `stdio` transport を追加し、HTTP transport なしでも MCP client から利用できるようにしました。
- `gyazo-mcp-server stdio --auth` を追加し、MCP login 非対応 client 向けの OAuth 認証準備を可能にしました。
- 実行時設定を `config.toml`、認証情報を `.env` へ分離しました。
- `stdio --auth` の完了処理を修正し、callback 後に終了しない不具合を解消しました。
- README と運用ガイドに `stdio` 利用手順、PAT の制約、client 設定例を追記しました。
