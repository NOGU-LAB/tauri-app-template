package handler

import (
	"backend/repository/memory"
	"backend/service"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
)

func newTestHandler() *UserHandler {
	return NewUserHandler(service.NewUserService(memory.NewUserRepository()))
}

func TestCreateAndGetUser(t *testing.T) {
	handler := newTestHandler()
	create := httptest.NewRequest(http.MethodPost, "/api/users", strings.NewReader(`{"name":"Alice","email":"alice@example.com"}`))
	create.Header.Set("Content-Type", "application/json")
	created := httptest.NewRecorder()
	handler.ServeHTTP(created, create)
	if created.Code != http.StatusCreated {
		t.Fatalf("create status = %d; body=%s", created.Code, created.Body.String())
	}

	get := httptest.NewRequest(http.MethodGet, "/api/users/1", nil)
	found := httptest.NewRecorder()
	handler.ServeHTTP(found, get)
	if found.Code != http.StatusOK || !strings.Contains(found.Body.String(), `"email":"alice@example.com"`) {
		t.Fatalf("unexpected get response: status=%d body=%s", found.Code, found.Body.String())
	}
}

func TestCreateUserRejectsInvalidRequests(t *testing.T) {
	tests := []struct {
		name        string
		contentType string
		body        string
		want        int
	}{
		{name: "missing content type", body: `{}`, want: http.StatusUnsupportedMediaType},
		{name: "unknown field", contentType: "application/json", body: `{"name":"A","email":"a@example.com","admin":true}`, want: http.StatusBadRequest},
		{name: "multiple values", contentType: "application/json", body: `{} {}`, want: http.StatusBadRequest},
		{name: "blank name", contentType: "application/json", body: `{"name":" ","email":"a@example.com"}`, want: http.StatusUnprocessableEntity},
		{name: "invalid email", contentType: "application/json", body: `{"name":"A","email":"invalid"}`, want: http.StatusUnprocessableEntity},
	}

	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			request := httptest.NewRequest(http.MethodPost, "/api/users", strings.NewReader(test.body))
			if test.contentType != "" {
				request.Header.Set("Content-Type", test.contentType)
			}
			response := httptest.NewRecorder()
			newTestHandler().ServeHTTP(response, request)
			if response.Code != test.want {
				t.Fatalf("status = %d, want %d; body=%s", response.Code, test.want, response.Body.String())
			}
		})
	}
}

func TestUserHandlerRejectsAmbiguousPath(t *testing.T) {
	request := httptest.NewRequest(http.MethodGet, "/api/users/1/extra", nil)
	response := httptest.NewRecorder()
	newTestHandler().ServeHTTP(response, request)
	if response.Code != http.StatusNotFound {
		t.Fatalf("status = %d, want %d", response.Code, http.StatusNotFound)
	}
}
