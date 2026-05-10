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

	// 親プロセス (Tauri) の死活監視: stdin が閉じられたら親が落ちたと見なし
	// 自己終了する。Tauri の CommandChild は明示 kill しない限り子を放置する
	// 仕様で、強制終了 (taskkill /F、Activity Monitor の強制終了等) では
	// Tauri の RunEvent::ExitRequested フックが呼ばれず Go プロセスがゾンビ
	// 化する。stdin の EOF は親のプロセス終了で必ず観測できるので、これで
	// 全ての終了経路に対して保険をかける。
	//
	// 通常運用では stdin に何も書かれないので Read はずっとブロックし、
	// 親死亡時に EOF (or pipe closed) で抜ける → exit。DEV_PORT で立てる
	// 開発モード (terminal 直起動) では stdin が tty なのでこのルートは
	// 触られない。
	if !isStdinTerminal() {
		go monitorParentStdin()
	}

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

// monitorParentStdin は親プロセスの死活を stdin の EOF で検知する。
// Tauri が sidecar を spawn すると stdin は親へのパイプになる。親が死ぬと
// パイプが閉じて Read が EOF / err を返すので、それを検知して自己終了する。
// 通常運用では誰も stdin に書かないので、このゴルーチンは EOF までブロック。
func monitorParentStdin() {
	buf := make([]byte, 256)
	for {
		n, err := os.Stdin.Read(buf)
		if err != nil {
			// EOF or pipe closed → 親死亡と判断して exit
			fmt.Fprintf(os.Stderr, "[lifecycle] parent stdin closed (%v), exiting\n", err)
			os.Exit(0)
		}
		// 何かデータが流れてきても無視 (現状 Tauri 側は stdin に書かない)
		_ = n
	}
}

// isStdinTerminal は stdin が tty (= 開発時に terminal から直起動) かを判定する。
// tty の場合は monitorParentStdin を回さない (terminal を閉じる前に
// アプリが落ちると困るし、そもそも親 = shell の死活監視は不要)。
//
// Tauri から sidecar として起動された場合は stdin がパイプなので tty で
// なくなり、この関数は false を返す → monitorParentStdin が起動する。
func isStdinTerminal() bool {
	fi, err := os.Stdin.Stat()
	if err != nil {
		return false
	}
	return (fi.Mode() & os.ModeCharDevice) != 0
}
