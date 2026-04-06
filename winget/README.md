# winget manifest スナップショット

このディレクトリには、microsoft/winget-pkgs に **新規申請したときの manifest ファイル** をスナップショットとして保管しています。

## 重要: リリース CI からは参照されません

リリース CI (`.github/workflows/release.yml` の `winget` ジョブ) は [vedantmgoyal9/winget-releaser](https://github.com/vedantmgoyal9/winget-releaser) を使用しており、内部で [Komac](https://github.com/russellbanks/Komac) が以下の流れで manifest を生成します。

1. winget-pkgs 上の既存 manifest を取得
2. 新しい version / installer URL / SHA256 を埋め込んで再生成
3. **常に最新の `ManifestVersion`** (執筆時点で `1.12.0`) で出力
4. fork (`ishii-masaki-646/winget-pkgs`) に push
5. microsoft/winget-pkgs に PR を送信

このディレクトリ配下の YAML ファイルは Komac から一切読まれません。**ここを編集しても次回リリースの自動 PR には反映されません。**

## 何のために残しているか

- **新規申請 PR (#355231) のスナップショット**: 初回登録時にどの形式で申請したかを履歴として残すため
- **手動操作の参考**: 将来 CI 自動化を外して手動 PR を出したくなったとき、また別パッケージを申請するときの雛形として
- **Komac の生成結果との比較**: 自動生成された manifest が想定と乖離していないかを確認するときの基準

## 自動連携に必要な設定

CI で winget-releaser を回すために必要なのは以下だけです。

| 項目 | 設定先 |
|------|--------|
| `WINGET_PAT` (Repository secret) | gyazo-mcp-server リポジトリの Settings → Secrets → Actions |
| fork (`ishii-masaki-646/winget-pkgs`) の存在 | 一度 fork ボタンを押すだけ |

PAT に必要な権限は `ishii-masaki-646/winget-pkgs` への Contents / Pull requests / Workflows の Read and write です。

## ファイル更新の指針

- **`ManifestVersion` の上げ忘れは気にしなくてよい** — Komac が常に最新を使うため
- **新しいフィールドを追加したい場合** — Komac が対応していれば自動で出力されるため、ここで管理する必要はない
- **どうしても CI 自動 PR の挙動を変えたい場合** — winget-releaser のオプションを `release.yml` 側で調整する (現状は `identifier` / `installers-regex` / `token` のみ指定)
