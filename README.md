# tauri-app

Tauri + Vite/React + Go のデスクトップアプリテンプレート。

## 技術スタック

| レイヤー | 技術 |
|---|---|
| フロントエンド | Vite + React + TypeScript |
| UIライブラリ | Bootstrap 5 + React-Bootstrap |
| アイコン | FontAwesome (react-fontawesome) |
| デスクトップ基盤 | Tauri v2 (Rust) |
| バックエンド | Go (サイドカー) |
| アーキテクチャ | Handler → Service → Repository |

---

## 全体構成図

```mermaid
graph TB
    subgraph Desktop App
        subgraph Tauri["Tauri v2 (Rust コア)"]
            TauriCore["lib.rs\nサイドカー起動\nイベント中継"]
        end

        subgraph Frontend["フロントエンド (Vite + React)"]
            Hook["useBackend hook\nポート受け取り"]
            UI["App.tsx\nBootstrap UI"]
        end

        subgraph GoBackend["Go バックエンド (サイドカー)"]
            Handler["Handler層\nHTTP入出力"]
            Service["Service層\nビジネスロジック"]
            Repository["Repository層\nデータアクセスI/F"]
            MemRepo["memory.Repository\n(現在の実装)"]
            DBRepo["sqlite / postgres\n(差し替え予定)"]
        end
    end

    TauriCore -->|"spawn + stdout監視"| GoBackend
    TauriCore -->|"backend-ready event\n(port番号)"| Hook
    Hook --> UI
    UI -->|"HTTP fetch\nlocalhost:PORT"| Handler
    Handler --> Service
    Service --> Repository
    Repository --> MemRepo
    Repository -.->|"将来差し替え"| DBRepo
```

---

## ポート検知フロー

```mermaid
sequenceDiagram
    participant T as Tauri (Rust)
    participant G as Go バックエンド
    participant R as React フロントエンド

    T->>G: サイドカーとして spawn
    G->>G: net.Listen(":0") で空きポート取得
    G-->>T: stdout: "PORT:53938"
    T->>T: stdout を監視・パース
    T-->>R: emit("backend-ready", 53938)
    R->>R: apiBase = "http://localhost:53938"
    R->>G: HTTP fetch (apiBase/api/...)
    G-->>R: JSON レスポンス
```

---

## Goバックエンド層構造

```mermaid
classDiagram
    class UserHandler {
        +service UserService
        +ServeHTTP(w, r)
        +getAllUsers()
        +getUser(id)
        +createUser()
        +deleteUser(id)
    }

    class UserService {
        +repo UserRepository
        +GetUser(id) User
        +GetAllUsers() []User
        +CreateUser(name, email) User
        +DeleteUser(id)
    }

    class UserRepository {
        <<interface>>
        +FindByID(id) User
        +FindAll() []User
        +Save(user)
        +Delete(id)
    }

    class InMemoryUserRepository {
        -data map
        +FindByID(id) User
        +FindAll() []User
        +Save(user)
        +Delete(id)
    }

    class SQLiteUserRepository {
        -db *sql.DB
        +FindByID(id) User
        +FindAll() []User
        +Save(user)
        +Delete(id)
    }

    UserHandler --> UserService
    UserService --> UserRepository
    UserRepository <|.. InMemoryUserRepository : implements
    UserRepository <|.. SQLiteUserRepository : implements (予定)
```

---

## 開発・本番のフロー比較

```mermaid
flowchart LR
    subgraph Dev["開発環境"]
        direction TB
        D1["./build-backend.sh\nGoバイナリ作成"] --> D2["npm run tauri dev\nTauri起動"]
        D2 --> D3["Vite dev server\nHMR有効"]
        D2 --> D4["Goサイドカー起動\n動的ポート"]

        subgraph HotReload["ホットリロード (Option A)"]
            H1["npm run backend:air\nDEV_PORT=8765 固定"] 
            H2["npm run tauri dev"]
        end
    end

    subgraph Prod["本番ビルド"]
        direction TB
        P1["npm run tauri build"] --> P2["build-backend.sh\n自動実行"]
        P2 --> P3["Go → バイナリ"]
        P1 --> P4["Vite build\nフロント最適化"]
        P3 --> P5["Tauri bundle\n全部同梱"]
        P4 --> P5
        P5 --> P6[".app / .dmg\n.exe / .msi"]
    end
```

---

## ディレクトリ構成

```
tauri-app/
├── src/                          # Vite + React フロントエンド
│   ├── hooks/useBackend.ts       # バックエンドポート受け取りフック
│   └── App.tsx                   # メインUI
├── src-tauri/                    # Tauri (Rust コア)
│   ├── src/lib.rs                # Goサイドカー起動 + ポートをフロントへ送信
│   ├── binaries/                 # ビルド済みGoバイナリ置き場
│   ├── capabilities/default.json # Tauriパーミッション設定
│   ├── tauri.conf.json           # Tauriアプリ設定（共通）
│   └── tauri.windows.conf.json   # Windows用設定上書き（beforeBuildCommand等）
├── backend/                      # Go バックエンド
│   ├── main.go                   # エントリポイント・空きポート検知
│   ├── server.go                 # ルーティング
│   ├── handler/                  # HTTPハンドラー
│   ├── service/                  # ビジネスロジック
│   ├── repository/               # データアクセスインターフェース
│   │   ├── memory/               # インメモリ実装
│   │   └── sqlite/               # SQLite実装
│   ├── infra/db.go               # SQLite接続・マイグレーション
│   ├── model/                    # データ構造体
│   └── .air.toml                 # air（ホットリロード）設定
├── build-backend.sh              # Goビルドスクリプト（macOS/Linux）
└── build-backend.ps1             # Goビルドスクリプト（Windows PowerShell）
```

---

## 設計指針

### 1. レイヤー分離（関心の分離）

各層は隣接する層のインターフェースのみに依存し、具体的な実装に依存しない。

```
Handler  →  Service  →  Repository (interface)
                              ↓
                         InMemoryRepository / SQLiteRepository / ...
```

- **Handler** はHTTPの入出力だけを担う。ビジネスロジックは持たない
- **Service** はビジネスロジックのみ。DBの種類を知らない
- **Repository** はインターフェースで定義。差し替えはmain.goのDI箇所だけ

### 2. DB未定でも開発を進める

`memory.InMemoryRepository` を差し込むことで、DBが決まる前からアプリケーションロジックを開発できる。
DBが決まったら `repository/sqlite/` などを追加し、`main.go` の1行を変えるだけで差し替え完了。

```go
// main.go — ここだけ変える
userRepo := memory.NewUserRepository()
// userRepo := sqlite.NewUserRepository(db)
```

### 3. 動的ポートによるポート衝突回避

本番環境では `net.Listen(":0")` でOSに空きポートを割り当てさせる。
固定ポートにしないことでポート競合が起きない。複数インスタンス起動も安全。

### 4. stdout経由のプロセス間通知

TauriサイドカーはGoプロセスのstdoutを直接読める。
ポート番号を `PORT:xxxxx` 形式でstdoutに出力し、Rustコアがキャッチしてフロントにeventを飛ばす。
HTTPサーバーや共有ファイルを使わないシンプルな起動通知。

### 5. フロントエンドはバックエンドの存在を意識しない

`useBackend` フックがポート受け取りとapiBaseの管理を隠蔽する。
各コンポーネントは `apiBase` を受け取るだけで、Tauriのイベント仕組みを知る必要がない。

### 6. 本番ビルドはコマンド1つ

`npm run tauri build` だけで以下がすべて自動実行される。
- Goバイナリのクロスコンパイル (`build-backend.sh`)
- Viteによるフロントエンドのバンドル
- TauriによるGoバイナリ同梱 + インストーラ生成

---

## セットアップ

### 必要なもの

- [Node.js](https://nodejs.org/) v18+
- [Go](https://go.dev/) v1.21+
- [Rust](https://www.rust-lang.org/) (rustup)
- [air](https://github.com/air-verse/air)（Goホットリロード、任意）: `go install github.com/air-verse/air@latest`

### インストール

```bash
npm install
```

---

## 開発

### 通常起動（Goはサイドカーとして自動起動）

```bash
# Goバイナリをビルド（初回・Go変更時に実行）
./build-backend.sh

# Tauriアプリ起動
npm run tauri dev
```

### Goホットリロードあり（Option A）

Goファイルを変えるたびに自動リビルド・再起動したい場合：

```bash
# ターミナル1: Go（air でホットリロード、固定ポート8765）
npm run backend:air

# ターミナル2: Tauri（Viteフロント + Rustコア）
npm run tauri dev
```

> **Note:** `backend:air` は `DEV_PORT=8765` の固定ポートで起動する。  
> Tauriサイドカーとは別プロセスになるため `backend-ready` イベントは飛ばない。  
> フロントは `http://localhost:8765` に直接fetchすること（開発時の割り切り）。

---

## 本番ビルド

### macOS / Linux

```bash
npm run tauri build
```

- `beforeBuildCommand` が `build-backend.sh` → `npm run build` を自動実行
- 出力先: `src-tauri/target/release/bundle/`
  - macOS: `.app` + `.dmg`

### Windows

```powershell
npm run tauri build
```

- `src-tauri/tauri.windows.conf.json` によって `beforeBuildCommand` が自動的に `pwsh` 経由に切り替わる
- Goバイナリのビルドには PowerShell 7 (`pwsh`) が必要
- 出力先: `src-tauri\target\release\bundle\`
  - `msi\tauri-app_x.x.x_x64_en-US.msi`（MSIインストーラー）
  - `nsis\tauri-app_x.x.x_x64-setup.exe`（NSISセットアップ）

#### Windowsの事前準備

```powershell
# winget で一括インストール
winget install GoLang.Go
winget install Rustlang.Rustup
winget install Microsoft.VisualStudio.2022.BuildTools --override "--add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"

# Rustツールチェーン（Visual Studio経由のMSVCが必要）
rustup toolchain install stable-x86_64-pc-windows-msvc
rustup default stable-x86_64-pc-windows-msvc

# Node.js依存関係インストール
npm install
```

> **注意:** macOS で作成した tar.gz を Windows に転送すると `._*` ファイル（macOSリソースフォーク）が混入することがある。  
> ビルド前に `Get-ChildItem -Recurse -Filter '._*' | Remove-Item -Force` で削除すること。

---

## DBの差し替え方

`backend/repository/` に新しい実装を追加し、`backend/main.go` のDI箇所を変更するだけ：

```go
// main.go
// userRepo := memory.NewUserRepository()   ← 現在
userRepo := sqlite.NewUserRepository(db)    // ← 差し替え後
```
