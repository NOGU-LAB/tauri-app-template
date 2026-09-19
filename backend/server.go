package main

import (
	"backend/handler"
	"encoding/json"
	"net/http"
)

func newServer(userHandler *handler.UserHandler) *http.ServeMux {
	mux := http.NewServeMux()
	mux.Handle("/api/users", userHandler)
	mux.Handle("/api/users/", userHandler)
	mux.HandleFunc("/api/health", func(w http.ResponseWriter, r *http.Request) {
		if r.Method != http.MethodGet {
			w.Header().Set("Allow", http.MethodGet)
			writeServerJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
			return
		}
		w.Header().Set("Content-Type", "application/json")
		_, _ = w.Write([]byte(`{"status":"ok"}`))
	})
	return mux
}

func writeServerJSONError(w http.ResponseWriter, status int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": message})
}
