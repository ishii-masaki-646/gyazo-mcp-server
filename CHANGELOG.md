# Changelog

## Unreleased

- `service` サブコマンドに `start` / `stop` / `restart` を追加しました。Linux は `systemctl --user start/stop/restart`、macOS は `launchctl load`/`unload` を経由してサービスを操作します。Windows は `schtasks /Run` で起動し、停止は `Get-NetTCPConnection -LocalPort <tcp_port> -State Listen` から OwningProcess の PID を取得し、その PID が `gyazo-mcp-server` であることを確認したうえで `Stop-Process -Id` で停止します。タスクスケジューラから `Start-Process` で本体を切り離して起動しているため `schtasks /End` ではラッパー PowerShell しか止められず、また `Get-Process -Name` ではサービス対象でない stdio モード等まで巻き込むため、`tcp_port` を listen しているプロセスを起点に厳密に特定する方式にしています。
- Windows の `service install` / `service uninstall` / `service status` で日本語等の非 ASCII 出力が文字化けする不具合を修正しました。生成する `.ps1` の先頭で `[Console]::OutputEncoding` と `$OutputEncoding` を UTF-8 に固定し、`service status` の `schtasks.exe` 呼び出しも PowerShell 経由で UTF-8 化してから受け取るようにしました。Windows PowerShell 5.x の標準出力が既定で OEM コードページ (日本語環境では CP932) になるため、Rust 側で UTF-8 として読むと文字化けしていました。
- Windows の `service uninstall` で、タスク登録を解除したあとに実行中の `gyazo-mcp-server.exe` (HTTP transport) を検出して警告を表示するようにしました。タスクスケジューラから `Start-Process` で本体を切り離して起動しているため、`Unregister-ScheduledTask` だけでは本体プロセスが停止せず、サービス登録を解除しても動き続ける問題に気付けるようにすることが目的です。検出は `Get-NetTCPConnection -State Listen` から OwningProcess を辿り、`ProcessName` が `gyazo-mcp-server` のものに限定するため、`stdio` モードで動いているインスタンスは巻き込みません。本体プロセスを自動で停止することはしません (別ポートで手動起動した HTTP インスタンス等、サービス管理対象でないプロセスを巻き込まないため)。検出されたプロセスの停止は `Stop-Process -Id <PID> -Force` を手動で実行してください。詳細は README の Windows uninstall 注意事項を参照してください。
- `service install` / `service uninstall` を冪等にしました。`service install` は既に登録されている場合に登録処理をスキップして案内メッセージのみ表示し、`service uninstall` は登録されていない場合に解除処理をスキップして案内メッセージのみ表示します。これまでは状態を確認せずに OS のインストール / アンインストール処理を実行していたため、二重登録や存在しないタスクの解除エラーが起きる場合がありました。
- `service start` / `service stop` / `service restart` を冪等にしました。`is_running()` を OS 別に追加し、`start` は既に起動中の場合に起動処理をスキップして案内メッセージのみ表示し、`stop` は既に停止中の場合に停止処理をスキップして案内メッセージのみ表示し、`restart` は停止中の場合に「起動のみ実行します」と表示してから起動します。検出は Linux で `systemctl --user is-active --quiet` の終了コード、macOS で `launchctl list` の PID 列、Windows で設定 TCP ポートを listen している `gyazo-mcp-server` プロセスの有無で行います。判定不能 (`Unknown`) の場合は OS 別ロジックへ流して本来のエラーを返します。これにより、`schtasks /Run` の二重発火による多重起動や、`launchctl unload` の「未ロード状態でのエラー」、`stop` の「未起動エラー」が起きなくなります。
- README に独立した `Uninstall` 節を追加しました。これまで `Install` 節と `Service` 節に分散していたアンインストール関連の記述を 1 か所に集約し、(1) 事前クリーニング (OS 別: Linux / macOS / Windows / Docker) と (2) パッケージのアンインストール (エコシステム別: Homebrew / winget / cargo / Docker) の 2 段階構成で案内するようにしました。設定ファイル (config.toml / .env / token.toml / mcp_sessions.toml) の保存先一覧と任意削除手順、Windows で `access denied` が出たときのリカバリ手順も同節に含めています。サービス登録を残したままバイナリを削除すると再ログイン時にタスクスケジューラ等が存在しないバイナリを起動しようとすること、Windows では実行中バイナリを削除できないためプロセスが残っているとアンインストール自体が失敗することも理由として明示しました。
- `gyazo_get_image` / `gyazo_list_images` / `gyazo_get_latest_image` / `gyazo_search` の戻り JSON に `resource_uri` フィールドを追加しました。値は MCP resource として登録済みの `gyazo-mcp:///{image_id}` 形式で、MCP `read_resource` に渡すと画像本体のバイナリを取得できます。
- 上記 4 ツールの `tool/list` description を、戻り値の `resource_uri` を MCP `read_resource` に渡すと画像本体のバイナリを取得できることが分かるよう拡張しました。エージェントが説明文だけから resource_uri の使い方に気付けるようにすることが目的です。
- `gyazo_get_oembed_metadata` の戻り JSON に `oembed_discovery_link` フィールドを追加しました。oEmbed spec 第 4 章で定義された discovery 用の `<link rel="alternate" type="application/json+oembed" href="https://api.gyazo.com/api/oembed?url=..." title="Image shared with Gyazo" />` 形式の HTML タグで、HTML ページの `<head>` に埋め込むと oEmbed 対応クライアント (クローラ等) が画像の埋め込み情報を発見できます。`href` の URL クエリ値は RFC 3986 unreserved 文字以外を percent encode するため、HTML 属性内に直接埋め込んでも安全です。
- 同ツールの戻り JSON に `img_tag_html` フィールドを追加しました。これは oEmbed spec の構造ではなく便利機能で、`url` / `width` / `height` と `alt` から組み立て済みの `<img src="..." width="..." height="..." alt="..." />` を返します。markdown / HTML にそのまま貼り付けて画像埋め込みに使えます。oEmbed spec の予約フィールド名 `html` との混同を避けるため `img_tag_html` という名前にしています。
- 同ツールに引数 `alt` を追加しました。`img_tag_html` の `alt` 属性に使われ、省略時は `"Gyazo image"` です。`alt` と `src` は HTML 属性向けに `&` / `<` / `>` / `"` を最小限エスケープします。
- 同ツールの description を更新し、`oembed_discovery_link` が oEmbed spec 準拠の discovery タグであることと、`img_tag_html` が spec 外の便利機能であること、title 等のリッチメタデータが必要な場合は `gyazo_get_image` を併用すべきことを明示しました。

## 0.5.1 - 2026-04-06

- 動的に登録された OAuth クライアント情報 (`registered_clients`) を `mcp_sessions.toml` に永続化するようにしました。サーバー再起動後に MCP クライアント側が OAuth 再検証フローに入ると `client_id` が未登録扱いになり、再認証ループに陥っていた不具合を解消しました。既存の `mcp_sessions.toml` は `#[serde(default)]` により後方互換のまま読み込めます。
- Windows の `service install` で生成する `.ps1` を UTF-8 BOM 付きで書き出すようにしました。Windows PowerShell 5.x が BOM なし UTF-8 を ANSI コードページとして解釈し、日本語等が文字化けまたはパースエラーになる問題を解消しました。
- Windows の `service install` で生成するタスクを `powershell.exe -WindowStyle Hidden -Command "Start-Process -WindowStyle Hidden ..."` 経由で起動するようにしました。タスクスケジューラから直接 EXE を起動するとフォアグラウンドのコンソールウィンドウが残ってしまう不具合を解消しました。
- CI の test ジョブを `ubuntu-latest` / `macos-latest` / `windows-latest` の matrix 構成に変更しました。これにより `service.rs` の OS 専用テストが CI で実行されるようになりました。
- リリース CI に winget 自動更新ジョブを追加しました。タグ push 時に `vedantmgoyal9/winget-releaser` 経由で `microsoft/winget-pkgs` に更新 PR を自動投稿します。
- リリース CI のビルド matrix に `aarch64-pc-windows-msvc` を追加しました。Snapdragon X 等の Windows on ARM 向けバイナリ (`gyazo-mcp-server-aarch64-pc-windows-msvc.zip`) も GitHub Release にアタッチされます。
- リリース CI に crates.io 自動 publish ジョブを追加しました。タグ push 時に `cargo publish` を自動実行します (`CARGO_REGISTRY_TOKEN` Repository secret が必要)。
- リリース CI に `verify` ジョブを最前段に追加しました。`v<VERSION>` タグと `Cargo.toml` の `version` が一致しない場合、build / release / docker / homebrew / winget / crates-io をすべて止めます。配信チャネル間の不整合を構造的に防ぐためのガードです。

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
