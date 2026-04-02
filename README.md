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

起動コマンドは次のとおりです。

```bash
gyazo-mcp-server
```

MCP endpoint は既定で `http://127.0.0.1:18449/mcp` です。
