# winget テスト計画

## 目的

`winget/` 配下の manifest を Windows 環境で検証し、`microsoft/winget-pkgs` への PR 作成前に最低限の動作確認を実施します。

対象バージョン:

- `0.5.0`

対象 manifest パス:

- `manifests/i/ishiimasaki646/gyazo-mcp-server/0.5.0`

## 前提条件

- Windows 10 / 11 環境で `winget` が利用可能であること
- `ishii-masaki-646/winget-pkgs` の `add-gyazo-mcp-server` ブランチを取得済みであること
- GitHub Releases 上に `v0.5.0` の Windows asset が存在すること
- 管理者権限は不要ですが、PowerShell を利用できること

## 確認項目

1. manifest の静的検証
2. manifest 経由のインストール
3. インストール後のバージョン確認
4. 実行ファイルの起動確認
5. アンインストールまたは後片付け

## 手順

### 1. リポジトリを準備

PowerShell で以下を実行します。

```powershell
cd <winget-pkgs の作業ディレクトリ>
git switch add-gyazo-mcp-server
git pull --ff-only origin add-gyazo-mcp-server
```

### 2. manifest を検証

```powershell
winget validate --manifest manifests\i\ishiimasaki646\gyazo-mcp-server\0.5.0
```

期待結果:

- validation が成功すること
- schema error、installer error、hash mismatch が出ないこと

### 3. manifest からインストール

```powershell
winget install --manifest manifests\i\ishiimasaki646\gyazo-mcp-server\0.5.0
```

期待結果:

- install が成功すること
- portable package として `gyazo-mcp-server.exe` が展開されること
- `PortableCommandAlias` により `gyazo-mcp-server` コマンドが利用可能になること

### 4. バージョン確認

新しい PowerShell を開くか、必要に応じて PATH を更新したあとで以下を実行します。

```powershell
gyazo-mcp-server --version
```

期待結果:

- `gyazo-mcp-server 0.5.0` が表示されること

### 5. 起動確認

認証情報がなくても起動確認だけは可能です。少なくとも help または version が応答することを確認します。

```powershell
gyazo-mcp-server --help
```

必要に応じて、HTTP サーバー起動まで確認する場合は別途設定ファイルと認証情報を用意したうえで実行します。

```powershell
gyazo-mcp-server
```

期待結果:

- `--help` が正常終了すること
- HTTP 起動確認を行う場合は、待ち受け開始まで到達すること

### 6. 後片付け

manifest 経由でインストールしたパッケージを削除します。

```powershell
winget uninstall ishiimasaki646.gyazo-mcp-server
```

期待結果:

- アンインストールが成功すること
- `gyazo-mcp-server` コマンドが残留しないこと

## 記録しておく項目

- 実行した Windows のバージョン
- `winget --version`
- `winget validate` の結果
- `winget install` の結果
- `gyazo-mcp-server --version` の出力
- 問題があった場合はエラーメッセージ全文

## PR 反映方針

すべて成功した場合は、`microsoft/winget-pkgs` 向け PR の checklist を以下のように更新します。

- `Have you validated your manifest locally with winget validate --manifest <path>?` をチェック
- `Have you tested your manifest locally with winget install --manifest <path>?` をチェック

失敗した場合は、manifest または release asset を修正してから再実施します。
