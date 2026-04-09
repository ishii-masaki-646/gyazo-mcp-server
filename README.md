# gyazo-mcp-server

[![crates.io](https://img.shields.io/crates/v/gyazo-mcp-server.svg)](https://crates.io/crates/gyazo-mcp-server)
[![docs.rs](https://img.shields.io/docsrs/gyazo-mcp-server)](https://docs.rs/gyazo-mcp-server)
[![Docker Hub](https://img.shields.io/docker/v/ishiimasaki646/gyazo-mcp-server?label=docker)](https://hub.docker.com/r/ishiimasaki646/gyazo-mcp-server)
[![Homebrew](https://img.shields.io/badge/homebrew-tap-orange)](https://github.com/ishii-masaki-646/homebrew-tap)
[![license](https://img.shields.io/crates/l/gyazo-mcp-server.svg)](https://github.com/ishii-masaki-646/gyazo-mcp-server)

Gyazo 向けのローカル MCP サーバーです。`/mcp` の streamable HTTP transport と、MCP 標準 login に準拠した認証 broker、さらに stdio transport を備えています。

## Features

- streamable HTTP transport と stdio transport の両対応
- MCP 標準 login に対応した OAuth broker
- Gyazo の `list / get / latest / search / upload / delete / oEmbed` tools
- `gyazo-mcp:///image_id` 形式の resources
- `config` / `env` サブコマンドによる対話的な初期設定と設定管理
- `service` サブコマンドによる OS サービスの常駐化 (Linux / macOS / Windows)
- Docker イメージによるコンテナ実行 (コンテナ内で自動的に `0.0.0.0` にバインド)

## Available Tools

- `gyazo_whoami`
- `gyazo_search`
- `gyazo_list_images`
- `gyazo_get_image`
- `gyazo_delete_image`
- `gyazo_get_latest_image`
- `gyazo_upload_image`
- `gyazo_get_oembed_metadata`

## Install

### Homebrew (macOS / Linux)

```bash
brew tap ishii-masaki-646/tap
brew install gyazo-mcp-server
```

### winget (Windows)

```powershell
winget install ishiimasaki646.gyazo-mcp-server
```

### Docker

```bash
docker run -d \
  -p 127.0.0.1:18449:18449 \
  -e GYAZO_MCP_PERSONAL_ACCESS_TOKEN=your-token \
  ishiimasaki646/gyazo-mcp-server
```

Docker Compose を使用する場合は、リポジトリの `docker-compose.yml` を参照してください。

### cargo install

crates.io からインストールする場合は、次のコマンドを使用します。

```bash
cargo install gyazo-mcp-server
```

インストール後のバイナリは通常 `~/.cargo/bin/gyazo-mcp-server` に配置されます。

ローカルソースからインストールする場合は、次のコマンドを使用します。

```bash
cargo install --path .
```

アンインストール手順は [Uninstall](#uninstall) を参照してください。

## Quick Start

インストール後、以下の手順で初期設定からサーバー起動まで行えます。

### 1. 基本設定と OAuth アプリケーション登録

```bash
gyazo-mcp-server config init
```

ポート番号などの基本設定を対話形式で行います。完了後、Gyazo の OAuth アプリケーション登録ページへのガイドが表示されます。

### 2. クレデンシャルの設定

```bash
gyazo-mcp-server env init
```

OAuth アプリケーション登録で取得した Client ID と Client Secret を設定します。

### 3. サーバーの起動

```bash
# HTTP transport で起動
gyazo-mcp-server

# または OS サービスとして常駐させる
gyazo-mcp-server service install
```

MCP endpoint は既定で `http://127.0.0.1:18449/mcp` です。ポートを変更した場合は `gyazo-mcp-server config get tcp_port` で確認してください。

### 簡易確認 (PAT)

OAuth の設定を省略して手早く試したい場合は、[Gyazo 開発者ページ](https://gyazo.com/api) で Personal Access Token を発行し、以下のように起動できます。

```bash
gyazo-mcp-server env set GYAZO_MCP_PERSONAL_ACCESS_TOKEN your-token
gyazo-mcp-server
```

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

## Config

設定ファイルはプラットフォームごとの標準ディレクトリに配置されます。

| OS | ディレクトリ |
|----|-------------|
| Linux | `~/.config/gyazo-mcp-server/` |
| macOS | `~/Library/Application Support/gyazo-mcp-server/` |
| Windows | `C:\Users\<user>\AppData\Roaming\gyazo-mcp-server\` |

動作設定は `config.toml`、認証情報は `.env` に分けて管理します。

`config.toml`:

```toml
tcp_port = 18449
bind_address = "127.0.0.1"
base_url = "http://127.0.0.1:18449"
oauth_callback_path = "/oauth/callback"
rust_log = "gyazo_mcp_server=info,rmcp=info"
```

`bind_address` はコンテナ内では自動的に `0.0.0.0` になります。`base_url` は OAuth metadata 等でクライアントに返す URL で、未指定時は `bind_address` から自動導出されます（`0.0.0.0` の場合は `127.0.0.1` にフォールバック）。LAN 向けに公開する場合は明示的に指定してください。

`.env`:

```env
GYAZO_MCP_OAUTH_CLIENT_ID=your-client-id
GYAZO_MCP_OAUTH_CLIENT_SECRET=your-client-secret
GYAZO_MCP_PERSONAL_ACCESS_TOKEN=your-personal-access-token
```

`GYAZO_MCP_PERSONAL_ACCESS_TOKEN` は簡易確認や個人利用向けの代替手段です。

### config / env サブコマンド

設定の確認・変更は `config` と `env` サブコマンドから行えます。

```bash
# 対話形式で初期設定（OAuth アプリ登録のガイド付き）
gyazo-mcp-server config init
gyazo-mcp-server env init

# 一覧表示
gyazo-mcp-server config show
gyazo-mcp-server env show

# 個別の取得・設定・削除
gyazo-mcp-server config get tcp_port
gyazo-mcp-server config set tcp_port 19000
gyazo-mcp-server config unset tcp_port

gyazo-mcp-server env get GYAZO_MCP_OAUTH_CLIENT_ID
gyazo-mcp-server env set GYAZO_MCP_OAUTH_CLIENT_ID your-id
gyazo-mcp-server env unset GYAZO_MCP_PERSONAL_ACCESS_TOKEN

# ファイルパスの確認
gyazo-mcp-server config path
gyazo-mcp-server env path
```

`env show` や `env get` でシークレット値を表示する際、出力先が TTY の場合はマスクされ、パイプの場合は生の値が出力されます。

### 設定ディレクトリの変更

設定ディレクトリを変更したい場合は、`config set config_dir` または `--config-dir` オプションを使用します。

```bash
# 永続的に変更（デフォルト位置の .env に保存される）
gyazo-mcp-server config set config_dir /path/to/custom

# 一時的に変更
gyazo-mcp-server --config-dir /path/to/custom config show
```

`config_dir` の永続化はデフォルト位置の `.env` に保存されます。これは、変更先の `.env` に書くと次回起動時にたどり着けなくなるためです。

### 環境変数による一時的な上書き

一時的に設定を上書きしたい場合は、環境変数でも指定できます。

```bash
GYAZO_MCP_TCP_PORT=14861 gyazo-mcp-server stdio --auth
```

## Gyazo OAuth Application

OAuth login を利用する場合は、Gyazo の開発者ページで OAuth Application を作成してください。

`config init` を実行すると、基本設定から OAuth アプリケーション登録、クレデンシャル設定までを対話形式でガイドします。

```bash
gyazo-mcp-server config init   # 基本設定 → OAuth アプリ登録のガイド
gyazo-mcp-server env init      # Client ID / Secret の設定
```

手動で設定する場合は、次の手順で行います。

1. [Gyazo](https://gyazo.com) にログインする
2. [アプリケーション登録ページ](https://gyazo.com/oauth/applications/new) を開く（未ログインの場合、ログイン後にキャプチャ一覧へ転送されることがあります。その場合は URL を直接開いてください）
3. Name に任意の名前（例: `Gyazo MCP Server`）を入力する
4. Callback URL に `http://127.0.0.1:<port><callback_path>` を登録する
5. 作成後、アプリ名をクリックして詳細を開き、`Client ID` と `Client Secret` を取得する
6. 取得した値を `gyazo-mcp-server env set` で設定する

既定値では Callback URL は `http://127.0.0.1:18449/oauth/callback` です。`config.toml` の `tcp_port` または `oauth_callback_path` を変更する場合は、Gyazo 側に登録する Callback URL も同じ値へ合わせてください。

まずローカルで動作確認したいだけであれば、Gyazo の開発者ページで発行できる Personal Access Token を `GYAZO_MCP_PERSONAL_ACCESS_TOKEN` に設定して利用することもできます。ただし、PAT で取得できるのは原則としてそのトークンを発行したユーザーに紐づく画像のみで、他ユーザーがアップロードした public 画像を取得する用途には向きません。また、PAT には OAuth login のようなユーザー認証文脈に基づく権限はないため、MCP login や user-scoped な認可の代替にはなりません。

## Run

HTTP transport で起動する場合は次のとおりです。

```bash
gyazo-mcp-server
```

MCP endpoint は既定で `http://127.0.0.1:18449/mcp` です。

Codex や Claude Code から利用する場合は、この endpoint を MCP server URL として登録し、MCP login を実行してください。

stdio transport で起動する場合は次のとおりです。

```bash
gyazo-mcp-server stdio
```

stdio transport では MCP login を行えないため、事前に次のいずれかを満たしておく必要があります。

- `GYAZO_MCP_PERSONAL_ACCESS_TOKEN` を設定しておく
- 先に HTTP transport 側で OAuth login を済ませ、保存済み token を作っておく

MCP login 非対応の client しか使えない場合は、先に one-shot の OAuth 認証だけ実行できます。

```bash
gyazo-mcp-server stdio --auth
```

このコマンドは一時的に callback 用のローカル HTTP サーバーを立てて、Gyazo OAuth を完了させたら token を保存して終了します。完了後は通常どおり `gyazo-mcp-server stdio` を client から起動してください。

`stdio --auth` を使う場合は、次の条件が必要です。

- Gyazo OAuth の authorize URL を開けるブラウザ、または同等の認可手段があること
- `http://127.0.0.1:<port><callback_path>` への callback を受け取れること

完全な非対話環境では、OAuth の代わりに `GYAZO_MCP_PERSONAL_ACCESS_TOKEN` を利用する方が現実的です。ただし、PAT は OAuth login のようなユーザー認証文脈の権限を持たないため、利用できる操作やアクセス範囲は限定されます。

## Service

HTTP transport のサーバーを OS サービスとして常駐させることができます。

```bash
# サービスとして登録・起動
gyazo-mcp-server service install

# サービスの状態を確認
gyazo-mcp-server service status

# サービス登録を解除
gyazo-mcp-server service uninstall
```

| OS | 登録先 |
|----|--------|
| Linux (systemd) | `~/.config/systemd/user/gyazo-mcp-server.service` |
| macOS (launchd) | `~/Library/LaunchAgents/com.gyazo.mcp-server.plist` |
| Windows | タスクスケジューラ (ログオン時に実行) |

systemd が存在しない Linux 環境や BSD 等では、手動でサービス設定を行う必要があります。

`--config-dir` で一時的に設定ディレクトリを変更した状態で `service install` を実行すると、常駐後のサービスが異なる設定を参照する可能性があるため、警告と確認が表示されます。常駐化する場合は、先に `config set config_dir` で永続化してください。

### Windows の `service uninstall` の注意事項

Windows のタスクスケジューラから登録したタスクは `powershell.exe -Command "Start-Process ..."` で `gyazo-mcp-server.exe` を別プロセスとして切り離して起動するため、`Unregister-ScheduledTask` だけでは本体プロセスが停止せず、サービス登録を解除した後もそのまま動き続けます。

`service uninstall` は本体プロセスを自動停止しません (別ポートで手動起動した HTTP インスタンス等、サービス管理対象でないプロセスを巻き込まないため)。そのかわり、登録解除のあとに `Get-NetTCPConnection -State Listen` から `ProcessName` が `gyazo-mcp-server` のプロセスを走査し、もし残っていれば PID と LocalPort を警告として表示します。`stdio` モードで動いているインスタンスは TCP ポートを bind しないため検出対象には含まれません。

警告に表示されたプロセスを停止する場合は、PowerShell から手動で次のように実行してください。

```powershell
Stop-Process -Id <PID> -Force
```

## Uninstall

クリーンにアンインストールするには、次の 2 段階で進めてください。

1. **事前クリーニング** — サービス登録の解除と、実行中プロセスの停止 (OS 別)
2. **パッケージのアンインストール** — 利用したパッケージマネージャの uninstall コマンド (エコシステム別)

サービス登録を残したままバイナリを削除すると、再ログイン時にタスクスケジューラや systemd / launchd が存在しないバイナリを起動しようとします。また Windows では実行中バイナリを削除できないため、プロセスが残っているとアンインストール自体が `access denied` で失敗します (Linux / macOS は実行中ファイルを削除しても OS が許す仕様なのでアンインストール自体は通りますが、いずれにせよプロセスは事前に止めておくのが安全です)。

### 1. 事前クリーニング (OS 別)

#### Linux / macOS

```bash
# サービス登録を解除 (登録している場合)
gyazo-mcp-server service uninstall

# 実行中の gyazo-mcp-server プロセスをすべて停止
pkill -x gyazo-mcp-server
```

#### Windows (PowerShell)

```powershell
# サービス登録を解除 (登録している場合)
gyazo-mcp-server service uninstall

# 実行中の gyazo-mcp-server プロセスをすべて停止
# (HTTP transport / stdio transport の両方を含めて停止します)
Get-Process gyazo-mcp-server -ErrorAction SilentlyContinue | Stop-Process -Force
```

#### Docker

不要です。Docker は OS のプロセス空間と分離されているため、コンテナを停止すれば内部の `gyazo-mcp-server` プロセスも一緒に終了します。手順 2 のコンテナ停止コマンドにそのまま進んでください。

### 2. パッケージのアンインストール (エコシステム別)

インストール時に使ったパッケージマネージャに合わせて、次のいずれかを実行してください。

#### Homebrew (macOS / Linux)

```bash
brew uninstall gyazo-mcp-server
# tap も外す場合:
brew untap ishii-masaki-646/tap
```

#### winget (Windows)

```powershell
winget uninstall ishiimasaki646.gyazo-mcp-server
```

#### cargo install

```bash
cargo uninstall gyazo-mcp-server
```

#### Docker

```bash
# 該当のコンテナを停止・削除
docker stop gyazo-mcp-server && docker rm gyazo-mcp-server
# イメージも削除する場合
docker rmi ishiimasaki646/gyazo-mcp-server
```

### 設定ファイルの削除 (任意)

`config.toml` / `.env` / OAuth token 等の設定ファイルは、上記いずれの手順でも削除されません。完全に消したい場合は次のディレクトリを手動で削除してください。

| OS | パス |
|----|------|
| Linux | `~/.config/gyazo-mcp-server/` |
| macOS | `~/Library/Application Support/gyazo-mcp-server/` |
| Windows | `%APPDATA%\gyazo-mcp-server\` |

### Windows でアンインストールが access denied で失敗する場合

事前クリーニングを行わずにパッケージマネージャの uninstall を直接実行してしまった場合、Windows では実行中の `gyazo-mcp-server.exe` を削除できないため `access denied` で失敗することがあります。その場合は上記「[1. 事前クリーニング (Windows)](#windows-powershell)」を実行してから、もう一度パッケージマネージャの uninstall を実行してください。

## HTTP Client Examples

### Codex CLI

```toml
[mcp_servers.gyazo]
type = "url"
url = "http://127.0.0.1:18449/mcp"
```

### Claude Code

```json
{
  "mcpServers": {
    "gyazo": {
      "type": "url",
      "url": "http://127.0.0.1:18449/mcp"
    }
  }
}
```

上記の URL は既定値です。`tcp_port` や `base_url` を変更した場合は `gyazo-mcp-server config show` で確認し、URL を合わせてください。

HTTP transport では事前に `gyazo-mcp-server` を起動しておき、クライアント側で MCP login を実行してください。

**Codex CLI の場合:**

```bash
codex mcp login gyazo
```

**Claude Code の場合:**

`/mcp` コマンドを実行し、Gyazo MCP Server の Authenticate を選択してください。

## Stdio Client Examples

### Codex CLI

Codex CLI では、たとえば次のように TOML で設定できます。

```toml
[mcp_servers.gyazo]
command = "/home/yourname/.cargo/bin/gyazo-mcp-server"
args = ["stdio"]
```

`stdio --auth` を先に一度実行して token を保存しておくか、`.env` に `GYAZO_MCP_PERSONAL_ACCESS_TOKEN` を設定してください。

### Claude Desktop / Claude Code

Claude Desktop や Claude Code 系では、たとえば次のように JSON で設定できます。

```json
{
  "mcpServers": {
    "gyazo": {
      "command": "/home/yourname/.cargo/bin/gyazo-mcp-server",
      "args": ["stdio"]
    }
  }
}
```

こちらも、事前に `gyazo-mcp-server stdio --auth` を一度実行して token を保存しておくか、`.env` に `GYAZO_MCP_PERSONAL_ACCESS_TOKEN` を設定してください。

設定管理には `config` / `env` サブコマンドも利用できます。詳しくは [Config](#config) セクションを参照してください。

## License

このリポジトリは `MIT OR Apache-2.0` で提供します。詳細は [LICENSE-MIT](LICENSE-MIT) と [LICENSE-APACHE](LICENSE-APACHE) を参照してください。
