package main

import (
	"backend/handler"
	"backend/infra"
	"backend/repository/memory"
	"backend/repository/sqlite"
	"backend/service"
	"flag"
	"fmt"
	"net"
	"net/http"
	"os"
	"strconv"
)

func main() {
	dbPath := flag.String("db", "", "SQLiteファイルパス（省略時はインメモリ）")
	flag.Parse()

	port, err := resolvePort()
	if err != nil {
		fmt.Fprintf(os.Stderr, "ポート取得エラー: %v\n", err)
		os.Exit(1)
	}

	// --db フラグがあればSQLite、なければインメモリ
	userService, err := buildUserService(*dbPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "初期化エラー: %v\n", err)
		os.Exit(1)
	}

	userHandler := handler.NewUserHandler(userService)
	mux := newServer(userHandler)

	// Tauriがstdoutを読んでフロントにポートを通知する
	fmt.Printf("PORT:%d\n", port)
	os.Stdout.Sync()

	addr := fmt.Sprintf(":%d", port)
	if err := http.ListenAndServe(addr, corsMiddleware(mux)); err != nil {
		fmt.Fprintf(os.Stderr, "サーバー起動エラー: %v\n", err)
		os.Exit(1)
	}
}

func buildUserService(dbPath string) (*service.UserService, error) {
	if dbPath == "" {
		return service.NewUserService(memory.NewUserRepository()), nil
	}
	db, err := infra.NewSQLite(dbPath)
	if err != nil {
		return nil, err
	}
	return service.NewUserService(sqlite.NewUserRepository(db)), nil
}

func resolvePort() (int, error) {
	if p := os.Getenv("DEV_PORT"); p != "" {
		port, err := strconv.Atoi(p)
		if err != nil {
			return 0, fmt.Errorf("DEV_PORT の値が不正です: %s", p)
		}
		return port, nil
	}
	return findFreePort()
}

func findFreePort() (int, error) {
	ln, err := net.Listen("tcp", ":0")
	if err != nil {
		return 0, err
	}
	defer ln.Close()
	return ln.Addr().(*net.TCPAddr).Port, nil
}

func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type")
		if r.Method == http.MethodOptions {
			w.WriteHeader(http.StatusNoContent)
			return
		}
		next.ServeHTTP(w, r)
	})
}
