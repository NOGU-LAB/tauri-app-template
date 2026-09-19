package handler

import (
	"backend/repository"
	"backend/service"
	"encoding/json"
	"errors"
	"io"
	"mime"
	"net/http"
	"strconv"
	"strings"
)

const maxRequestBodySize = 1 << 20

type UserHandler struct {
	service *service.UserService
}

func NewUserHandler(service *service.UserService) *UserHandler {
	return &UserHandler{service: service}
}

func (h *UserHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")

	if r.URL.Path != "/api/users" && r.URL.Path != "/api/users/" {
		idPart := strings.TrimPrefix(r.URL.Path, "/api/users/")
		if idPart == r.URL.Path || idPart == "" || strings.Contains(idPart, "/") {
			writeJSONError(w, http.StatusNotFound, "not found")
			return
		}
		id, err := strconv.Atoi(idPart)
		if err != nil || id <= 0 {
			writeJSONError(w, http.StatusBadRequest, "invalid id")
			return
		}
		switch r.Method {
		case http.MethodGet:
			h.getUser(w, r, id)
		case http.MethodDelete:
			h.deleteUser(w, r, id)
		default:
			w.Header().Set("Allow", "GET, DELETE")
			writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
		}
		return
	}

	switch r.Method {
	case http.MethodGet:
		h.getAllUsers(w, r)
	case http.MethodPost:
		h.createUser(w, r)
	default:
		w.Header().Set("Allow", "GET, POST")
		writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
	}
}

func (h *UserHandler) getAllUsers(w http.ResponseWriter, r *http.Request) {
	users, err := h.service.GetAllUsers()
	if err != nil {
		writeJSONError(w, http.StatusInternalServerError, "internal server error")
		return
	}
	_ = json.NewEncoder(w).Encode(users)
}

func (h *UserHandler) getUser(w http.ResponseWriter, r *http.Request, id int) {
	user, err := h.service.GetUser(id)
	if err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeJSONError(w, http.StatusNotFound, "user not found")
		} else {
			writeJSONError(w, http.StatusInternalServerError, "internal server error")
		}
		return
	}
	_ = json.NewEncoder(w).Encode(user)
}

func (h *UserHandler) createUser(w http.ResponseWriter, r *http.Request) {
	contentType, _, err := mime.ParseMediaType(r.Header.Get("Content-Type"))
	if err != nil || contentType != "application/json" {
		writeJSONError(w, http.StatusUnsupportedMediaType, "content type must be application/json")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, maxRequestBodySize)
	var req struct {
		Name  string `json:"name"`
		Email string `json:"email"`
	}
	decoder := json.NewDecoder(r.Body)
	decoder.DisallowUnknownFields()
	if err := decoder.Decode(&req); err != nil {
		writeJSONError(w, http.StatusBadRequest, "invalid request body")
		return
	}
	if err := decoder.Decode(&struct{}{}); err != io.EOF {
		writeJSONError(w, http.StatusBadRequest, "request body must contain one JSON object")
		return
	}
	user, err := h.service.CreateUser(req.Name, req.Email)
	if err != nil {
		if errors.Is(err, service.ErrInvalidName) || errors.Is(err, service.ErrInvalidEmail) {
			writeJSONError(w, http.StatusUnprocessableEntity, err.Error())
		} else {
			writeJSONError(w, http.StatusInternalServerError, "internal server error")
		}
		return
	}
	w.WriteHeader(http.StatusCreated)
	_ = json.NewEncoder(w).Encode(user)
}

func (h *UserHandler) deleteUser(w http.ResponseWriter, r *http.Request, id int) {
	if err := h.service.DeleteUser(id); err != nil {
		if errors.Is(err, repository.ErrNotFound) {
			writeJSONError(w, http.StatusNotFound, "user not found")
		} else {
			writeJSONError(w, http.StatusInternalServerError, "internal server error")
		}
		return
	}
	w.WriteHeader(http.StatusNoContent)
}

func writeJSONError(w http.ResponseWriter, status int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	_ = json.NewEncoder(w).Encode(map[string]string{"error": message})
}
