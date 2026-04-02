# Official Gyazo MCP メモ

確認日: 2026-04-02

## 公式実装の概要
- 公式リポジトリ: `nota/gyazo-mcp-server`
- npm パッケージ: `@notainc/gyazo-mcp-server`
- 現行の公式実装は TypeScript ベース
- README では `GYAZO_ACCESS_TOKEN` を使う `stdio` サーバーとして案内されている

## 公式実装がサポートしているもの

### Tools
- `gyazo_search`
  - Gyazo 上のキャプチャを全文検索する
  - 主要パラメータ: `query`, `page`, `per`
- `gyazo_image`
  - 画像 ID または Gyazo URL から画像とメタデータを取得する
  - 主要パラメータ: `id_or_url`
- `gyazo_latest_image`
  - 最新のアップロード画像を取得する
  - 主要パラメータ: `name`
- `gyazo_upload`
  - Base64 画像を Gyazo にアップロードする
  - 主要パラメータ: `imageData`, `app`, `description`, `refererUrl`, `title`

### Resources
- README には `gyazo-mcp://` URI の Resources もあると記載されている
- 含まれる情報の例:
  - 元画像
  - メタデータ (`title`, `description`, `app`, `url`)
  - OCR データ

## 実装メモ
- 公式実装は `PAT` ベースで、MCP 標準の login broker までは備えていない
- このリポジトリでは、公式の 4 tools をベースラインとしてそろえる方針が自然である
- 追加の差別化要素としては、`MCP login` と `whoami` のような認証確認系ツールが考えられる

## Sources
- https://github.com/nota/gyazo-mcp-server
- https://hub.docker.com/mcp/server/gyazo/overview
