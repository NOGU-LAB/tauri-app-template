# 新しいバックエンドリソースを追加する

引数: $ARGUMENTS（例: `product`, `order`）

以下のすべてのファイルを作成・更新して、`$ARGUMENTS` リソースのCRUD機能をバックエンドに追加してください。

## 作成するファイル

### 1. モデル `backend/model/{resource}.go`
```go
package model

type {Resource} struct {
    ID   int    `json:"id"`
    // TODO: フィールドを追加
}
```

### 2. Repositoryインターフェース `backend/repository/{resource}_repository.go`
```go
package repository

type {Resource}Repository interface {
    FindByID(id int) (*model.{Resource}, error)
    FindAll() ([]*model.{Resource}, error)
    Save(r *model.{Resource}) error
    Delete(id int) error
}
```

### 3. インメモリ実装 `backend/repository/memory/{resource}_repository.go`
- `sync.RWMutex` で並行安全にする
- `Save`: ID=0 なら採番して挿入、ID>0 なら更新
- `Delete`: 存在しなければ `errors.New("{resource} not found")`

### 4. SQLite実装 `backend/repository/sqlite/{resource}_repository.go`
- `database/sql` を使用（`modernc.org/sqlite` ドライバー）
- `FindByID`: `sql.ErrNoRows` → `errors.New("{resource} not found")`
- `Save`: ID=0 なら INSERT + LastInsertId、ID>0 なら UPDATE

### 5. サービス `backend/service/{resource}_service.go`
```go
package service

type {Resource}Service struct {
    repo repository.{Resource}Repository
}

func New{Resource}Service(repo repository.{Resource}Repository) *{Resource}Service

// メソッド: Get{Resource}, GetAll{Resource}s, Create{Resource}, Delete{Resource}
```

### 6. ハンドラー `backend/handler/{resource}_handler.go`
- `ServeHTTP` でパス・メソッドをswitch分岐
- `/api/{resource}s` GET → 一覧、POST → 作成
- `/api/{resource}s/{id}` GET → 1件、DELETE → 削除
- リクエストボディは `json.NewDecoder(r.Body).Decode()`
- レスポンスは `json.NewEncoder(w).Encode()`

## 更新するファイル

### `backend/infra/db.go`
`NewSQLite()` の自動マイグレーションに `{resource}s` テーブルのCREATE TABLE IF NOT EXISTS を追記。

### `backend/server.go`
`newServer()` にハンドラーを追加:
```go
mux.Handle("/api/{resource}s", {resource}Handler)
mux.Handle("/api/{resource}s/", {resource}Handler)
```

### `backend/main.go`
- `build{Resource}Service()` 関数を追加（memory/sqlite切り替え）
- `main()` で `{resource}Handler := handler.New{Resource}Handler({resource}Service)` を追加

## 注意点
- User リソース（`backend/repository/memory/user_repository.go` など）を参照パターンとして使うこと
- `modernc.org/sqlite` はCGO不要なのでビルド設定変更は不要
- 追加後は `./build-backend.sh` でGoバイナリを再ビルドすること
