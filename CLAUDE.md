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

## ハマりどころ (実プロジェクトで踏んだ罠)

このテンプレートを土台に派生したアプリ (marathon-admin-tool 等) で実際に
踏んだ罠を、再発防止のために共有する。

### 1. `productName` を日本語にすると WiX (MSI) ビルドが落ちる

WiX の `light.exe` は productName が非 ASCII の場合に内部で失敗する。
本テンプレートはこの問題を避けるため、デフォルトの `bundle.targets` を
**MSI を含まない明示リスト** にしてある:

```jsonc
// src-tauri/tauri.conf.json (template デフォルト)
"bundle": {
  "targets": ["nsis", "deb", "appimage", "rpm", "app", "dmg"]
}
```

NSIS は日本語 productName でもファイル名にそのまま使えるので、
`マラソンシステム管理ツール_0.1.0_x64-setup.exe` のような出力になる。

**MSI が必要な場合** (企業配布など) は productName を ASCII に保ち、
`"targets"` に `"msi"` を加える。両立させたい場合は MSI 用に英名の
`Window.title` 別系統を保つ等の工夫が必要。

### 2. macOS の Overlay タイトルバーで自前タイトルを描画する場合

`titleBarStyle: "Overlay"` + `hiddenTitle: true` で OS のタイトル文字を消し、
HTML 側で中央配置のカスタムタイトル (`<div class="custom-title-bar">`) を
重ねる構成は **macOS のときだけ描画** すること。Windows / Linux では OS が
ネイティブタイトルバーを別領域として管理しているので、HTML カスタムタイトルを
出すと画面上部に 2 段になって重複する。

加えて、カスタムタイトル分の高さ (例: 32px) を確保するために
`body { padding-top: 32px }` や `.app-layout { height: calc(100vh - 32px) }`
を入れる時は、**両方を一緒に macOS 限定スコープにする**こと
(`body.has-custom-title-bar { ... }` 等で揃える)。片方だけ条件付きにすると、
Windows で下端 32px の白い空白 / macOS でフッタ見切れ、のどちらかが起きる。

### 3. Brother QL 系 USB プリンタは Windows で `nusb::claim_interface` が落ちる

USB Printer Class デバイスを Windows カーネルドライバ (`usbprint.sys`) が
排他保持しているため、libusb 系 (nusb / rusb) から `claim_interface(0)` すると
ACCESS DENIED になる。Windows では下記いずれかで回避:

- **Spooler RAW 経路を使う**: Windows プリンタとして登録済みのデバイスへ
  `OpenPrinterW` → `StartDocPrinterW` (Datatype="RAW") → `WritePrinter` で
  ラスタコマンドをそのまま流す。`windows = "0.61"` の features に
  `"Win32_Graphics_Printing"` と `"Win32_Security"` を有効化する。
- Zadig で WinUSB / libusbK にドライバを差し替える (= OS 印刷から外れるので非推奨)。

macOS は CUPS が claim していなければ libusb で取れるので nusb で問題ない。
**OS で経路分岐するのが現実的な落としどころ**。

### 4. Git の行コード問題で `git pull` が blocked

Windows で checkout したファイルが CRLF になり、macOS 側の LF と差分が出る。
その状態で `git pull` すると "local changes would be overwritten" でブロック。
本テンプレートは `.gitattributes` で行コードを明示しているので普通は起きないが、
他リポジトリから移植する時は要注意。

### 5. Cargo / webview2-com / windows-rs のバージョン整合

Tauri 内部で使っている `webview2-com` / `windows` のバージョンと、自前で
追加する依存 (印刷 / WebView2 PrintToPdf 等) のバージョンを **完全に揃える**こと。
バージョンが違うと別 crate インスタンスとして並行展開され、`Param<PCWSTR>` 等の
trait 実装が共有されずビルドが通らない。確認は `cargo tree | grep windows`。

```toml
# 例: Tauri 2.10 系では webview2-com 0.38 / windows 0.61 系
[target.'cfg(target_os = "windows")'.dependencies]
webview2-com = "0.38"
windows = { version = "0.61", features = [...] }
```
