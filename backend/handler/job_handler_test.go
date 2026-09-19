package handler

import (
	"backend/service"
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestJobHandlerStartsGetsAndCancelsJob(t *testing.T) {
	handler := NewJobHandler(service.NewJobService(50 * time.Millisecond))
	start := httptest.NewRequest(http.MethodPost, "/api/jobs", strings.NewReader(`{"fileName":"data.json","content":"[{\"id\":1}]"}`))
	start.Header.Set("Content-Type", "application/json")
	started := httptest.NewRecorder()
	handler.ServeHTTP(started, start)
	if started.Code != http.StatusAccepted {
		t.Fatalf("start status = %d; body=%s", started.Code, started.Body.String())
	}

	idStart := strings.Index(started.Body.String(), `"id":"`) + len(`"id":"`)
	idEnd := strings.Index(started.Body.String()[idStart:], `"`) + idStart
	if idStart < len(`"id":"`) || idEnd <= idStart {
		t.Fatalf("job id missing: %s", started.Body.String())
	}
	id := started.Body.String()[idStart:idEnd]

	cancel := httptest.NewRequest(http.MethodDelete, "/api/jobs/"+id, nil)
	cancelled := httptest.NewRecorder()
	handler.ServeHTTP(cancelled, cancel)
	if cancelled.Code != http.StatusOK || !strings.Contains(cancelled.Body.String(), `"status":"cancelled"`) {
		t.Fatalf("cancel response: status=%d body=%s", cancelled.Code, cancelled.Body.String())
	}
}

func TestJobHandlerRejectsInvalidData(t *testing.T) {
	handler := NewJobHandler(service.NewJobService(time.Millisecond))
	request := httptest.NewRequest(http.MethodPost, "/api/jobs", strings.NewReader(`{"fileName":"data.json","content":"not-json"}`))
	request.Header.Set("Content-Type", "application/json")
	response := httptest.NewRecorder()
	handler.ServeHTTP(response, request)
	if response.Code != http.StatusUnprocessableEntity {
		t.Fatalf("status = %d; body=%s", response.Code, response.Body.String())
	}
}
