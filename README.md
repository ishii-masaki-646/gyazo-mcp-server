# gyazo-mcp-server

Gyazo 向けのローカル HTTP MCP サーバーよ。`/mcp` の streamable HTTP transport と、MCP 標準 login に寄せた認証 broker を持ってるの。

## Build

開発ビルド:

```bash
cargo build
```

リリースビルド:

```bash
cargo build --release
```

生成されるバイナリはこれね。

```text
target/release/gyazo-mcp-server
```

## Install

ローカルの Cargo bin へグローバルインストールするならこれでいいわ。

```bash
cargo install --path .
```

インストール後のバイナリは通常 `~/.cargo/bin/gyazo-mcp-server` に入るの。

アンインストールはこれね。

```bash
cargo uninstall gyazo-mcp-server
```

## Config

設定は `~/.config/gyazo-mcp-server/.env` に置く想定よ。

```env
GYAZO_MCP_TCP_PORT=18449
GYAZO_MCP_OAUTH_CALLBACK_PATH=/oauth/callback
GYAZO_MCP_OAUTH_CLIENT_ID=your-client-id
GYAZO_MCP_OAUTH_CLIENT_SECRET=your-client-secret
GYAZO_MCP_PERSONAL_ACCESS_TOKEN=your-personal-access-token
```

起動はこれ。

```bash
gyazo-mcp-server
```

MCP endpoint は既定で `http://127.0.0.1:18449/mcp` になるわ。
