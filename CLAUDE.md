# cliip-show 開発ガイド

## テスト

コードを変更したあとは **UT と VRT の両方** を必ず確認すること。

```bash
# UT（ユニットテスト）
cargo test

# VRT（ビジュアルリグレッションテスト）
./scripts/visual_regression.sh
```

### 運用ルール
- 通常の変更: 上記2つがすべて通ることを確認してからPRを出す
- 意図したUI変更（HUD外観の変更など）: VRTのベースラインを更新する
  ```bash
  ./scripts/visual_regression.sh --update
  ```

## モジュール構成

| ファイル | 役割 |
|---|---|
| `src/main.rs` | エントリポイント（`fn main` のみ） |
| `src/cli.rs` | CLIフラグの処理（`--help`, `--config`, `--render-hud-png` など） |
| `src/config.rs` | 設定の型・読み書き・パース・`--config` サブコマンド |
| `src/hud.rs` | HUDウィンドウ生成・レイアウト計算・描画 |
| `src/app.rs` | AppDelegate・クリップボード監視・フェードアニメーション |
| `src/text.rs` | テキスト切り詰め処理 |
| `src/png.rs` | PNG生成・差分計算（VRT用） |
| `src/objc_helpers.rs` | NSString変換ユーティリティ |
| `src/error.rs` | `AppError` 型定義 |
