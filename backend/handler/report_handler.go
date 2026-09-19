package handler

import (
	"backend/service"
	"encoding/json"
	"net/http"
	"time"
)

type ReportHandler struct {
	now func() time.Time
}

func NewReportHandler() *ReportHandler {
	return &ReportHandler{now: time.Now}
}

func (h *ReportHandler) ServeHTTP(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	if r.Method != http.MethodGet {
		w.Header().Set("Allow", http.MethodGet)
		writeJSONError(w, http.StatusMethodNotAllowed, "method not allowed")
		return
	}
	_ = json.NewEncoder(w).Encode(service.GenerateSampleReport(h.now()))
}
