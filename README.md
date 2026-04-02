# gyazo-mcp-server

Gyazo 向けのローカル HTTP MCP サーバーです。`/mcp` の streamable HTTP transport と、MCP 標準 login に準拠した認証 broker を備えています。

## Build

開発ビルド:

```bash
cargo build
```

リリースビルド:

```bash
cargo build --release
```

生成されるバイナリは次のとおりです。

```text
target/release/gyazo-mcp-server
```

## Install

ローカルの Cargo bin へグローバルインストールする場合は、次のコマンドを使用します。

```bash
cargo install --path .
```

インストール後のバイナリは通常 `~/.cargo/bin/gyazo-mcp-server` に配置されます。

アンインストールは次のとおりです。

```bash
cargo uninstall gyazo-mcp-server
```

## Config

設定は `~/.config/gyazo-mcp-server/.env` に配置する想定です。

```env
GYAZO_MCP_TCP_PORT=18449
GYAZO_MCP_OAUTH_CALLBACK_PATH=/oauth/callback
GYAZO_MCP_OAUTH_CLIENT_ID=your-client-id
GYAZO_MCP_OAUTH_CLIENT_SECRET=your-client-secret
GYAZO_MCP_PERSONAL_ACCESS_TOKEN=your-personal-access-token
```

## Gyazo OAuth Application

OAuth login を利用する場合は、Gyazo の開発者ページで OAuth Application を作成してください。

1. [Gyazo のアプリケーション登録ページ](https://gyazo.com/oauth/applications) を開く
2. Application を作成し、`client_id` と `client_secret` を取得する
3. `redirect_uri` には `http://127.0.0.1:<port><callback_path>` を登録する
4. 取得した値を `GYAZO_MCP_OAUTH_CLIENT_ID` と `GYAZO_MCP_OAUTH_CLIENT_SECRET` に設定する

既定値では `redirect_uri` は `http://127.0.0.1:18449/oauth/callback` です。`GYAZO_MCP_TCP_PORT` または `GYAZO_MCP_OAUTH_CALLBACK_PATH` を変更する場合は、Gyazo 側に登録する `redirect_uri` も同じ値へ合わせてください。

まずローカルで動作確認したいだけであれば、Gyazo の開発者ページで発行できる Personal Access Token を `GYAZO_MCP_PERSONAL_ACCESS_TOKEN` に設定して利用することもできます。ただし、PAT で取得できるのは原則としてそのトークンを発行したユーザーに紐づく画像のみで、他ユーザーがアップロードした public 画像を取得する用途には向きません。

起動コマンドは次のとおりです。

```bash
gyazo-mcp-server
```

MCP endpoint は既定で `http://127.0.0.1:18449/mcp` です。

## License

このリポジトリは `MIT OR Apache-2.0` で提供します。詳細は [LICENSE-MIT](LICENSE-MIT) と [LICENSE-APACHE](LICENSE-APACHE) を参照してください。
