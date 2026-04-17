package main

import (
	"backend/handler"
	"net/http"
)

func newServer(userHandler *handler.UserHandler) *http.ServeMux {
	mux := http.NewServeMux()
	mux.Handle("/api/users", userHandler)
	mux.Handle("/api/users/", userHandler)
	mux.HandleFunc("/api/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.Write([]byte(`{"status":"ok"}`))
	})
	return mux
}
