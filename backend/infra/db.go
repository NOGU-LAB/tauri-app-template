package infra

import (
	"database/sql"
	"fmt"
	_ "modernc.org/sqlite"
)

// NewSQLite はSQLiteのDB接続を初期化してマイグレーションを実行する
func NewSQLite(dbPath string) (*sql.DB, error) {
	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		return nil, fmt.Errorf("DB接続失敗: %w", err)
	}
	db.SetMaxOpenConns(1)
	if err := db.Ping(); err != nil {
		db.Close()
		return nil, fmt.Errorf("DB接続確認失敗: %w", err)
	}
	if _, err := db.Exec(`PRAGMA busy_timeout = 5000; PRAGMA foreign_keys = ON`); err != nil {
		db.Close()
		return nil, fmt.Errorf("DB設定失敗: %w", err)
	}
	if err := migrate(db); err != nil {
		db.Close()
		return nil, fmt.Errorf("マイグレーション失敗: %w", err)
	}
	return db, nil
}

func migrate(db *sql.DB) error {
	_, err := db.Exec(`
		CREATE TABLE IF NOT EXISTS users (
			id    INTEGER PRIMARY KEY AUTOINCREMENT,
			name  TEXT NOT NULL,
			email TEXT NOT NULL
		)
	`)
	return err
}
