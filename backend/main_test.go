package main

import (
	"net"
	"net/http"
	"net/http/httptest"
	"testing"
)

func TestResolveListenerUsesLoopback(t *testing.T) {
	t.Setenv("DEV_PORT", "")
	listener, err := resolveListener()
	if err != nil {
		t.Fatal(err)
	}
	defer listener.Close()

	address := listener.Addr().(*net.TCPAddr)
	if !address.IP.IsLoopback() {
		t.Fatalf("listener must use loopback, got %s", address.IP)
	}
}

func TestResolveListenerRejectsInvalidPort(t *testing.T) {
	t.Setenv("DEV_PORT", "70000")
	if _, err := resolveListener(); err == nil {
		t.Fatal("expected invalid port error")
	}
}

func TestSecurityMiddleware(t *testing.T) {
	const token = "test-token"
	handler := securityMiddleware(http.HandlerFunc(func(w http.ResponseWriter, _ *http.Request) {
		w.WriteHeader(http.StatusOK)
	}), token)

	tests := []struct {
		name   string
		origin string
		token  string
		method string
		want   int
	}{
		{name: "valid token", token: token, method: http.MethodGet, want: http.StatusOK},
		{name: "missing token", method: http.MethodGet, want: http.StatusUnauthorized},
		{name: "invalid token", token: "wrong", method: http.MethodGet, want: http.StatusUnauthorized},
		{name: "trusted origin", origin: "tauri://localhost", token: token, method: http.MethodGet, want: http.StatusOK},
		{name: "untrusted origin", origin: "https://example.com", token: token, method: http.MethodGet, want: http.StatusForbidden},
		{name: "trusted preflight", origin: "http://localhost:1420", method: http.MethodOptions, want: http.StatusNoContent},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			request := httptest.NewRequest(test.method, "/api/health", nil)
			if test.origin != "" {
				request.Header.Set("Origin", test.origin)
			}
			if test.token != "" {
				request.Header.Set(backendTokenHeader, test.token)
			}
			response := httptest.NewRecorder()
			handler.ServeHTTP(response, request)
			if response.Code != test.want {
				t.Fatalf("status = %d, want %d; body=%s", response.Code, test.want, response.Body.String())
			}
		})
	}
}

func TestNewAuthTokenIsRandom(t *testing.T) {
	first, err := newAuthToken()
	if err != nil {
		t.Fatal(err)
	}
	second, err := newAuthToken()
	if err != nil {
		t.Fatal(err)
	}
	if first == second || len(first) < 40 {
		t.Fatalf("tokens must be long and unique: %q %q", first, second)
	}
}

func TestResolveAuthTokenAllowsDevelopmentOverride(t *testing.T) {
	t.Setenv("DEV_PORT", "8765")
	t.Setenv("DEV_BACKEND_TOKEN", "local-development-token")
	token, err := resolveAuthToken()
	if err != nil {
		t.Fatal(err)
	}
	if token != "local-development-token" {
		t.Fatalf("token = %q", token)
	}
}
