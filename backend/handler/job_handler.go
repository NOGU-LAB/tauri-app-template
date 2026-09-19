package handler

import (
	"backend/service"
	"encoding/json"
	"errors"
	"io"
	"mime"
	"net/http"
	"strings"
)

// JSON文字列化で引用符などがエスケープされても、5MBの入力を受け取れる上限。
const maxJobRequestBodySize = 12 << 20

type JobHandler struct {
	service *service.JobService
}

func NewJobHandler(service *service.JobService) *JobHandler {
	return &JobHandler{service: service}
}

func (h *JobHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	if r.URL.Path == "/api/jobs" || r.URL.Path == "/api/jobs/" {
		if r.Method != http.MethodPost {
			w.Header().Set("Allow", http.MethodPost)
			writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
			return
		}
		h.start(w, r)
		return
	}

	id := strings.TrimPrefix(r.URL.Path, "/api/jobs/")
	if id == r.URL.Path || id == "" || strings.Contains(id, "/") {
		writeJSONError(w, http.StatusNotFound, "not found")
		return
	}
	switch r.Method {
	case http.MethodGet:
		job, err := h.service.Get(id)
		if errors.Is(err, service.ErrJobNotFound) {
			writeJSONError(w, http.StatusNotFound, "job not found")
			return
		}
		_ = json.NewEncoder(w).Encode(job)
	case http.MethodDelete:
		job, err := h.service.Cancel(id)
		if errors.Is(err, service.ErrJobNotFound) {
			writeJSONError(w, http.StatusNotFound, "job not found")
			return
		}
		if errors.Is(err, service.ErrJobNotRunning) {
			writeJSONError(w, http.StatusConflict, "job is not running")
			return
		}
		_ = json.NewEncoder(w).Encode(job)
	default:
		w.Header().Set("Allow", "GET, DELETE")
		writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
	}
}

func (h *JobHandler) start(w http.ResponseWriter, r *http.Request) {
	contentType, _, err := mime.ParseMediaType(r.Header.Get("Content-Type"))
	if err != nil || contentType != "application/json" {
		writeJSONError(w, http.StatusUnsupportedMediaType, "content type must be application/json")
		return
	}
	r.Body = http.MaxBytesReader(w, r.Body, maxJobRequestBodySize)
	var req struct {
		FileName string `json:"fileName"`
		Content  string `json:"content"`
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
	job, err := h.service.Start(req.FileName, req.Content)
	if err != nil {
		if errors.Is(err, service.ErrInvalidJobFile) || errors.Is(err, service.ErrJobTooLarge) || strings.Contains(err.Error(), "解析") || strings.Contains(err.Error(), "JSON") || strings.Contains(err.Error(), "データがありません") {
			writeJSONError(w, http.StatusUnprocessableEntity, err.Error())
		} else {
			writeJSONError(w, http.StatusInternalServerError, "internal server error")
		}
		return
	}
	w.WriteHeader(http.StatusAccepted)
	_ = json.NewEncoder(w).Encode(job)
}
