package handler

import (
	"net/http"
	"net/http/httptest"
	"strings"
	"testing"
	"time"
)

func TestReportHandler(t *testing.T) {
	handler := NewReportHandler()
	handler.now = func() time.Time { return time.Unix(0, 0).UTC() }
	recorder := httptest.NewRecorder()
	handler.ServeHTTP(recorder, httptest.NewRequest(http.MethodGet, "/api/reports/sample", nil))
	if recorder.Code != http.StatusOK || !strings.Contains(recorder.Body.String(), `"pdfBase64"`) {
		t.Fatalf("unexpected response: %d %s", recorder.Code, recorder.Body.String())
	}
}
