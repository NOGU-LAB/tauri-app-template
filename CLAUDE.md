# tauri-app プロジェクト指針

## 技術スタック

- **フロントエンド**: Vite + React + TypeScript + Bootstrap 5 + FontAwesome
- **デスクトップ基盤**: Tauri v2 (Rust)
- **バックエンド**: Go（Tauriサイドカー）

## バックエンドアーキテクチャ

```
Handler → Service → Repository (interface)
                         ↓
              memory / sqlite / （将来のDB）
```

- `backend/model/` — データ構造体（ビジネスロジックなし）
- `backend/repository/` — Repository インターフェース定義
- `backend/repository/memory/` — インメモリ実装
- `backend/repository/sqlite/` — SQLite実装（`modernc.org/sqlite`、CGOなし）
- `backend/service/` — ビジネスロジック（Repository interfaceだけに依存）
- `backend/handler/` — HTTPハンドラー（Service呼び出し、JSON入出力）
- `backend/server.go` — ルーティング定義
- `backend/infra/db.go` — SQLite接続・テーブル自動マイグレーション
- `backend/main.go` — DI・サイドカー起動（`--db` フラグでDBパスを受け取る）

## 設計ルール

### Repository
- インターフェースは `backend/repository/{entity}_repository.go` に定義
- `memory/` と `sqlite/` の両方を必ず実装する
- メソッド名: `FindByID`, `FindAll`, `Save`, `Delete`（新規はID=0, 更新はID>0で `Save` を共用）

### Service
- `repository.XxxRepository` インターフェースのみ受け取る（具体実装を知らない）
- エラーは `errors.New("xxx not found")` など簡潔に返す

### Handler
- `ServeHTTP` でパスとメソッドをswitch分岐
- 正常系: 適切なHTTPステータス + JSON
- エラー系: `http.Error` でメッセージ返却

### DB（SQLite）
- テーブル追加は `backend/infra/db.go` の `NewSQLite()` 内にマイグレーションSQLを追記
- `modernc.org/sqlite` はCGO不要のため Windows/Mac クロスビルドが可能

### main.go のDI
- `buildXxxService()` 関数でmemory/sqliteを切り替える
- `--db` フラグなし → インメモリ（テスト・開発用）
- `--db` フラグあり → SQLite（本番）

## ビルドコマンド

```bash
# Goバイナリをビルド（Go変更時に必要）
./build-backend.sh          # macOS/Linux
./build-backend.ps1         # Windows (pwsh)

# 開発起動
npm run tauri dev

# 本番ビルド（Go + Vite + Tauri bundle を自動実行）
npm run tauri build
```

## Tauriとの連携

- Goサイドカーは `net.Listen(":0")` で空きポートを取得し、`PORT:xxxxx` をstdoutに出力
- Rust (`lib.rs`) がstdoutを監視してポートをReactに `backend-ready` イベントで通知
- Reactの `useBackend` フックがポートを受け取り `apiBase` を設定
- DBパスは Tauri の `app_data_dir()` → `--db` フラグ経由でGoに渡す

## Windows対応

- `src-tauri/tauri.windows.conf.json` で `beforeBuildCommand` を `pwsh` に上書き
- macOSのtar.gzをWindowsに転送した場合、`._*` ファイルを削除してからビルドすること
